use aws_config::{ConfigLoader, meta::region::RegionProviderChain};
use aws_sdk_ec2::{
    Client, Error,
    client::Waiters,
    error::SdkError,
    operation::create_vpc::CreateVpcError,
    types::{
        Instance, InstanceStateName, InstanceType, IpPermission, IpRange, KeyPairInfo,
        ResourceType, SecurityGroup, Subnet, Tag, TagSpecification, Vpc,
        builders::{IpPermissionBuilder, TagSpecificationBuilder},
    },
};
use clap::builder::OsStr;
use std::{collections::HashMap, fs, ops::Deref, path::PathBuf, time::Duration};

use crate::config::AppConfig;

pub const KEY_NAME: &str = "the-beez-kees";

pub async fn mk_client(ac: &AppConfig) -> Result<Client, Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2024_03_28()).await;
    Ok(Client::new(&config))
}

fn create_tag_spec(ac: &AppConfig, rt: ResourceType) -> TagSpecification {
    TagSpecification::builder()
        .resource_type(rt)
        .tags(
            Tag::builder()
                .key("Name")
                .value(ac.tag_name.clone())
                .build(),
        )
        .build()
}

// VPCs

pub async fn create_vpc(client: &Client, ac: &AppConfig) -> Result<Vpc, Error> {
    let tag_specifications = create_tag_spec(ac, ResourceType::Vpc);
    println!("[create_vpc]");
    println!("[create_vpc] tags: {:?}", tag_specifications);

    let response = client
        .create_vpc()
        .cidr_block("10.0.0.0/16")
        .tag_specifications(tag_specifications)
        .send()
        .await?;

    let vpc = response.vpc.as_ref().unwrap();
    let vpc_id = vpc.vpc_id().unwrap();
    println!("[create_vpc] success {:?}", vpc_id);

    Ok(vpc.clone())
}

pub async fn describe_vpc(client: &Client, vpc_id: &str) -> Result<Vec<Vpc>, Error> {
    println!("[describe_vpc] vpc_id {}", vpc_id);

    let Ok(response) = client.describe_vpcs().vpc_ids(vpc_id).send().await else {
        panic!("[describe_vpc] error");
    };

    let vpcs = response.vpcs.unwrap();
    println!("[describe_vpc] success {:?}", vpcs.len());
    Ok(vpcs)
}

// Subnets

pub async fn create_subnet(client: &Client, ac: &AppConfig) -> Result<Subnet, Error> {
    let vpc_id = ac.vpc_id.as_ref().unwrap();
    let tag_specifications = create_tag_spec(ac, ResourceType::Subnet);
    println!("[create_subnet] vpc_id {}", &ac.vpc_id.as_ref().unwrap());
    println!("[create_subnet] tags: {:?}", tag_specifications);

    let response = client
        .create_subnet()
        .vpc_id(vpc_id.clone())
        .cidr_block("10.0.1.0/24")
        .tag_specifications(tag_specifications)
        //.availability_zone("us-east-1a")
        .send()
        .await?;

    let subnet = response.subnet.unwrap();
    let subnet_id = subnet.subnet_id().unwrap();
    println!("[create_subnet] success {:?}", subnet_id);

    Ok(subnet.clone())
}

pub async fn describe_subnet(client: &Client, subnet_id: &str) -> Result<Vec<Subnet>, Error> {
    println!("[describe_subnet] subnet_id {}", subnet_id);

    let Ok(response) = client.describe_subnets().subnet_ids(subnet_id).send().await else {
        panic!("[describe_subnet] error");
    };

    let subnets = response.subnets.unwrap();
    println!("[describe_subnet] success {:?}", subnets.len());
    Ok(subnets)
}

// Security Groups

pub async fn create_security_group(client: &Client, ac: &AppConfig) -> Result<String, Error> {
    let vpc_id = ac.vpc_id.as_ref().unwrap();
    let tag_specifications = create_tag_spec(ac, ResourceType::SecurityGroup);
    let ssh_cidr_block = ac.ssh_cidr_block.as_ref().unwrap();
    println!("[create_security_group] vpc_id {:?}", vpc_id);
    println!("[create_security_group] tags {:?}", tag_specifications);
    println!("[create_security_group] ssh cidr {:?}", ssh_cidr_block);

    let response = client
        .create_security_group()
        .vpc_id(vpc_id.clone())
        .group_name("allow-ssh")
        .description("Allow SSH inbound traffic")
        .tag_specifications(tag_specifications)
        .send()
        .await?;

    let sg_id = response.group_id.unwrap();

    println!("[create_security_group] success {:?}", sg_id);

    // Add ingress rule to allow SSH
    client
        .authorize_security_group_ingress()
        .group_id(&sg_id)
        .set_ip_permissions(Some(vec![
            IpPermission::builder()
                .ip_protocol("tcp")
                .from_port(22)
                .to_port(22)
                .ip_ranges(
                    IpRange::builder()
                        .cidr_ip(ssh_cidr_block.to_string())
                        .build(),
                )
                .build(),
        ]))
        .send()
        .await?;

    println!("[create_security_group] ingress");

    // Add egress rule to allow all outbound traffic
    client
        .authorize_security_group_egress()
        .group_id(&sg_id)
        .set_ip_permissions(Some(vec![
            IpPermission::builder()
                .ip_protocol("tcp")
                .from_port(0)
                .to_port(0)
                .ip_ranges(IpRange::builder().cidr_ip("0.0.0.0/0").build())
                .build(),
        ]))
        .send()
        .await?;

    println!("[create_security_group] egress");

    Ok(sg_id.clone())
}

pub async fn describe_security_group(
    client: &Client,
    security_group_id: &str,
) -> Result<Vec<SecurityGroup>, Error> {
    println!(
        "[describe_security_group] security_group_id {}",
        security_group_id
    );

    let Ok(response) = client
        .describe_security_groups()
        .group_ids(security_group_id)
        .send()
        .await
    else {
        panic!("[describe_security_group] error");
    };

    let security_groups = response.security_groups.unwrap();
    println!(
        "[describe_security_group] success {:?}",
        security_groups.len()
    );
    Ok(security_groups)
}

// Key Pairs

pub async fn import_key_pair(
    client: &Client,
    ac: &AppConfig,
    key_name: &str,
) -> Result<String, Error> {
    println!("[import_key_pair] name {}", key_name);
    println!("[import_key_pair] key_file {}", ac.key_file.clone());

    let tag_specifications = create_tag_spec(ac, ResourceType::KeyPair);

    let key_path = PathBuf::from(ac.key_file.clone());
    println!(
        "[import_key_pair] key_file {:?}",
        fs::canonicalize(&key_path)
    );

    let key_material = match std::fs::read_to_string(ac.key_file.clone()) {
        Ok(key_material) => key_material,
        Err(e) => panic!("[key material] read_to_string\n{}", e),
    };
    println!("[key material] loaded");

    let key_blob = aws_sdk_ec2::primitives::Blob::new(key_material);

    let Ok(response) = client
        .import_key_pair()
        .key_name(key_name)
        .public_key_material(key_blob)
        .tag_specifications(tag_specifications)
        .send()
        .await
    else {
        panic!("[key pair] Waaaah!");
    };

    let key_id = response.key_pair_id.unwrap();
    println!("[key material] success {:?}", key_id);
    Ok(key_id)
}

pub async fn describe_key_pair(client: &Client, key_name: &str) -> Result<Vec<KeyPairInfo>, Error> {
    println!("[describe_key_pair] key_name {}", key_name);

    match client.describe_key_pairs().key_names(key_name).send().await {
        Ok(response) => {
            let key_pairs = response.key_pairs.unwrap();
            println!("[describe_key_pair] success {:?}", key_pairs.len());
            Ok(key_pairs)
        }
        Err(e) => {
            println!("[describe_key_pair] no key found");
            Ok(Vec::new())
        }
    }
}

// Instances

pub async fn create_instances(
    client: &Client,
    vpc_id: &str,
    subnet_id: &str,
    sg_id: &str,
    ac: &AppConfig,
) -> Result<Vec<String>, Error> {
    println!("[create_instances]");
    let tag_specifications = create_tag_spec(ac, ResourceType::Instance);

    let response = match client
        .run_instances()
        .instance_type(InstanceType::T2Micro)
        .image_id(ac.ami.clone().unwrap())
        .key_name(KEY_NAME)
        .subnet_id(subnet_id)
        .security_group_ids(sg_id)
        .tag_specifications(tag_specifications.clone())
        .min_count(ac.num_beez)
        .max_count(ac.num_beez)
        .send()
        .await
    {
        Ok(instances) => instances,
        Err(e) => panic!("[create_instances] ERROR create {:?}", e),
    };

    if response.instances().is_empty() {
        panic!("[create_instances] ERROR no instances created");
    }

    let instance_ids = response
        .instances
        .unwrap()
        .iter()
        .map(|i| i.instance_id.clone().unwrap())
        .collect();

    Ok(instance_ids)
}

pub async fn wait_for_instances(client: &Client, instance_ids: &[String]) -> Result<(), Error> {
    loop {
        // Wait for a while before checking again (e.g., 30 seconds)
        println!("[wait_for_instances] waiting 30 seconds");
        tokio::time::sleep(Duration::from_secs(30)).await;

        let resp = match client
            .describe_instances()
            .instance_ids(instance_ids.join(","))
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => continue, // panic!("[wait_for_instances] ERROR {:?}", e),
        };

        let mut all_online = true;

        for reservation in resp.reservations.unwrap_or_default() {
            for instance in reservation.instances.unwrap_or_default() {
                if let Some(public_ip) = instance.clone().public_ip_address {
                    println!(
                        "[wait_for_instances] instance {} has public IP: {}",
                        instance.clone().instance_id.unwrap_or_default(),
                        public_ip
                    );
                } else {
                    println!(
                        "[wait_for_instances] instance {} has not a public IP",
                        instance.clone().instance_id.unwrap_or_default()
                    );
                }

                if let Some(state_name) = instance.state.clone().unwrap().name() {
                    if state_name != &InstanceStateName::Running {
                        all_online = false; // Mark as false if any instance is not running
                        println!(
                            "[wait_for_instances] instance {} is not online, current state: {:?}",
                            instance.instance_id.unwrap_or_default(),
                            state_name
                        );
                    } else {
                        println!(
                            "[wait_for_instances] instance {} is online!",
                            instance.instance_id.unwrap_or_default()
                        );
                    }
                }
            }
        }

        // If all instances are online, break the loop
        if all_online {
            println!("[wait_for_instances] all instances are online!");
            break;
        }
    }

    Ok(())
}
