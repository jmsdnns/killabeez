use aws_config::{ConfigLoader, meta::region::RegionProviderChain};
use aws_sdk_ec2::{
    Client, Error,
    client::Waiters,
    error::SdkError,
    operation::create_vpc::CreateVpcError,
    types::{
        AttributeBooleanValue, Filter, Instance, InstanceStateName, InstanceType, IpPermission,
        IpRange, KeyPairInfo, NetworkInterface, ResourceType, SecurityGroup, Subnet, Tag,
        TagSpecification, Vpc,
        builders::{IpPermissionBuilder, NetworkInterfaceBuilder, TagSpecificationBuilder},
    },
};
use clap::builder::OsStr;
use std::{collections::HashMap, fs, ops::Deref, path::PathBuf, ptr::read, time::Duration};

use crate::{config::SwarmConfig, scenarios::AWSNetwork};

pub const KEY_NAME: &str = "the-beez-kees";

pub async fn mk_client() -> Result<Client, Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2024_03_28()).await;
    Ok(Client::new(&config))
}

fn create_tag_spec(sc: &SwarmConfig, rt: ResourceType) -> TagSpecification {
    TagSpecification::builder()
        .resource_type(rt)
        .tags(
            Tag::builder()
                .key("Name")
                .value(sc.tag_name.clone())
                .build(),
        )
        .build()
}

// VPCs

pub async fn create_vpc(client: &Client, sc: &SwarmConfig) -> Result<Vpc, Error> {
    let tag_specifications = create_tag_spec(sc, ResourceType::Vpc);
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

pub async fn create_subnet(client: &Client, sc: &SwarmConfig) -> Result<Subnet, Error> {
    let vpc_id = sc.vpc_id.as_ref().unwrap();
    let tag_specifications = create_tag_spec(sc, ResourceType::Subnet);
    println!("[create_subnet] vpc_id {}", &sc.vpc_id.as_ref().unwrap());
    println!("[create_subnet] tags: {:?}", tag_specifications);

    let response = client
        .create_subnet()
        .vpc_id(vpc_id.clone())
        .cidr_block("10.0.1.0/24")
        .tag_specifications(tag_specifications)
        .send()
        .await?;

    let subnet = response.subnet.unwrap();
    let subnet_id = subnet.subnet_id().unwrap();
    println!("[create_subnet] success {:?}", subnet_id);

    client
        .modify_subnet_attribute()
        .subnet_id(subnet_id)
        .map_public_ip_on_launch(AttributeBooleanValue::builder().value(true).build())
        .send()
        .await?;
    println!("[create_subnet] maps public ip on launch");

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

pub async fn create_security_group(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
    let vpc_id = sc.vpc_id.as_ref().unwrap();
    let tag_specifications = create_tag_spec(sc, ResourceType::SecurityGroup);
    let ssh_cidr_block = sc.ssh_cidr_block.as_ref().unwrap();
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
    sc: &SwarmConfig,
    key_name: &str,
) -> Result<String, Error> {
    println!("[import_key_pair] name {}", key_name);
    println!("[import_key_pair] key_file {}", sc.key_file.clone());

    let tag_specifications = create_tag_spec(sc, ResourceType::KeyPair);

    let key_path = PathBuf::from(sc.key_file.clone());
    println!(
        "[import_key_pair] key_file {:?}",
        fs::canonicalize(&key_path)
    );

    let key_material = match std::fs::read_to_string(sc.key_file.clone()) {
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

#[derive(Debug, Clone)]
pub struct Bee {
    pub id: String,
    pub ip: Option<String>,
}

pub async fn create_instances(
    client: &Client,
    sc: &SwarmConfig,
    network: &AWSNetwork,
    fill_count: Option<i32>,
) -> Result<Vec<Bee>, Error> {
    println!("[create_instances]");
    let tag_specifications = create_tag_spec(sc, ResourceType::Instance);

    let new_beez = match fill_count {
        Some(count) => count,
        None => sc.num_beez,
    };

    let response = match client
        .run_instances()
        .instance_type(InstanceType::T2Micro)
        .image_id(sc.ami.clone().unwrap())
        .key_name(KEY_NAME)
        .subnet_id(network.subnet_id.clone())
        .security_group_ids(network.security_group_id.clone())
        .tag_specifications(tag_specifications.clone())
        .min_count(new_beez)
        .max_count(new_beez)
        .send()
        .await
    {
        Ok(instances) => instances,
        Err(e) => panic!("[create_instances] ERROR create {:?}", e),
    };

    if response.instances().is_empty() {
        panic!("[create_instances] ERROR no instances created");
    }

    let instances = response
        .instances
        .unwrap()
        .iter()
        .map(|i| Bee {
            id: i.instance_id.clone().unwrap(),
            ip: i.public_ip_address.clone(),
        })
        .collect();

    Ok(instances)
}

pub enum BeeLoader {
    Ids(Vec<Bee>),
    Tagged(String),
}

pub async fn describe_instances(client: &Client, loader: BeeLoader) -> Result<Vec<Bee>, Error> {
    println!("[describe_instances]");

    let request = match loader {
        BeeLoader::Ids(ids) => match ids.len() {
            0 => None,
            _ => Some(
                client
                    .describe_instances()
                    .set_instance_ids(Some(
                        ids.iter().map(|b| b.id.clone()).collect::<Vec<String>>(),
                    ))
                    .send(),
            ),
        },
        BeeLoader::Tagged(tag) => {
            let filter = Filter::builder().name("tag:Name").values(tag).build();
            Some(client.describe_instances().filters(filter).send())
        }
    };

    match request {
        None => Ok(Vec::new()),
        Some(loader) => match loader.await {
            Ok(response) => Ok(response
                .reservations
                .clone()
                .unwrap_or_default()
                .iter()
                .flat_map(|r| {
                    r.instances
                        .clone()
                        .unwrap_or_default()
                        .iter()
                        .filter(|&i| {
                            matches!(
                                i.clone().state.clone().unwrap().name.unwrap(),
                                InstanceStateName::Running
                            )
                        })
                        .flat_map(|i| {
                            Some(Bee {
                                id: i.instance_id.clone().unwrap(),
                                ip: i.public_ip_address.clone(),
                            })
                        })
                        .collect::<Vec<Bee>>()
                })
                .collect::<Vec<Bee>>()),
            Err(e) => panic!("[load_tagged] ERROR {}", e),
        },
    }
}

pub async fn describe_tagged(client: &Client, sc: &SwarmConfig) -> Result<Vec<Bee>, Error> {
    println!("[load_tagged]");
    describe_instances(client, BeeLoader::Tagged(sc.tag_name.clone())).await
}

pub async fn wait_for_running(client: &Client, beez: Vec<Bee>) -> Result<Vec<Bee>, Error> {
    println!("[wait_for_running]");
    loop {
        let running_beez = describe_instances(client, BeeLoader::Ids(beez.clone())).await;
        let running_beez = running_beez.unwrap();

        // return Ok when counts match
        let delta = beez.len() - running_beez.len();
        if delta == 0 {
            return Ok(running_beez.clone());
        }
        println!("[wait_for_running] waiting 15 seconds for {} beez", delta);
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}
