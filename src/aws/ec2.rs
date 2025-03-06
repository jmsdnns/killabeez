use aws_config::{ConfigLoader, meta::region::RegionProviderChain};
use aws_sdk_ec2::{
    Client, Error,
    client::Waiters,
    error::SdkError,
    operation::create_vpc::CreateVpcError,
    types,
    types::builders::{IpPermissionBuilder, NetworkInterfaceBuilder, TagSpecificationBuilder},
};
use clap::builder::OsStr;
use std::{collections::HashMap, fs, ops::Deref, path::PathBuf, ptr::read, time::Duration};

use crate::{config::SwarmConfig, scenarios::AWSNetwork};

pub const KEY_NAME: &str = "the-beez-kees";

pub async fn mk_client() -> Result<Client, Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2024_03_28()).await;
    Ok(Client::new(&config))
}

fn create_tag_spec(sc: &SwarmConfig, rt: types::ResourceType) -> types::TagSpecification {
    types::TagSpecification::builder()
        .resource_type(rt)
        .tags(
            types::Tag::builder()
                .key("Name")
                .value(sc.tag_name.clone())
                .build(),
        )
        .build()
}

/// Beez can be loaded via id list or a resource tag
pub enum ResourceMatcher {
    Id(Vec<String>),
    Tagged(String),
}

fn create_tag_filter(tag: &str) -> types::Filter {
    types::Filter::builder()
        .name("tag:Name")
        .values(tag)
        .build()
}

// VPCs

pub struct VPC {}
impl VPC {
    pub async fn create(client: &Client, sc: &SwarmConfig) -> Result<types::Vpc, Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Vpc);
        println!("[VPC.create]");
        println!("[VPC.create] tags: {:?}", tag_specifications);

        let response = client
            .create_vpc()
            .cidr_block("10.0.0.0/16")
            .tag_specifications(tag_specifications)
            .send()
            .await?;

        let vpc = response.vpc.as_ref().unwrap();
        let vpc_id = vpc.vpc_id().unwrap();
        println!("[VPC.create] success {:?}", vpc_id);

        Ok(vpc.clone())
    }

    pub async fn describe(
        client: &Client,
        matcher: ResourceMatcher,
    ) -> Result<Vec<types::Vpc>, Error> {
        let r = client.describe_vpcs();
        let request = match matcher {
            ResourceMatcher::Id(vpc_ids) => match vpc_ids.len() {
                0 => None,
                _ => Some(r.set_vpc_ids(Some(vpc_ids.clone())).send()),
            },
            ResourceMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag)).send()),
        };

        match request {
            None => Ok(Vec::new()),
            Some(response) => match response.await {
                Ok(response) => {
                    let vpcs = response.vpcs.clone().unwrap();
                    match vpcs.len() {
                        0 => Ok(Vec::new()),
                        _ => Ok(vpcs.clone()),
                    }
                }
                Err(e) => panic!("[VPC.describe] ERROR\n{}", e),
            },
        }
    }

    pub async fn delete(client: &Client, vpc_id: &str) -> Result<(), Error> {
        match client
            .delete_vpc()
            .set_vpc_id(Some(vpc_id.to_string()))
            .send()
            .await
        {
            Ok(response) => {
                println!("[VPC.delete] success {}", vpc_id);
                Ok(())
            }
            Err(e) => {
                panic!("[VPC.delete] ERROR {}", e);
            }
        }
    }
}

// Subnets

pub struct Subnet {}
impl Subnet {
    pub async fn create(client: &Client, sc: &SwarmConfig) -> Result<types::Subnet, Error> {
        let vpc_id = sc.vpc_id.as_ref().unwrap();
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Subnet);
        println!("[Subnet.create] vpc_id {}", &sc.vpc_id.as_ref().unwrap());
        println!("[Subnet.create] tags: {:?}", tag_specifications);

        let response = client
            .create_subnet()
            .vpc_id(vpc_id.clone())
            .cidr_block("10.0.1.0/24")
            .tag_specifications(tag_specifications)
            .send()
            .await?;

        let subnet = response.subnet.unwrap();
        let subnet_id = subnet.subnet_id().unwrap();
        println!("[Subnet.create] success {:?}", subnet_id);

        client
            .modify_subnet_attribute()
            .subnet_id(subnet_id)
            .map_public_ip_on_launch(types::AttributeBooleanValue::builder().value(true).build())
            .send()
            .await?;
        println!("[Subnet.create] maps public ip on launch");

        Ok(subnet.clone())
    }

    pub async fn describe(
        client: &Client,
        matcher: ResourceMatcher,
    ) -> Result<Vec<types::Subnet>, Error> {
        let request = client.describe_subnets();
        let request = match matcher {
            ResourceMatcher::Id(subnet_ids) => match subnet_ids.len() {
                0 => None,
                _ => Some(request.set_subnet_ids(Some(subnet_ids.clone())).send()),
            },
            ResourceMatcher::Tagged(tag) => Some(request.filters(create_tag_filter(&tag)).send()),
        };

        match request {
            None => Ok(Vec::new()),
            Some(response) => match response.await {
                Ok(response) => {
                    let subnets = response.subnets.clone().unwrap();
                    match subnets.len() {
                        0 => Ok(Vec::new()),
                        _ => Ok(subnets.clone()),
                    }
                }
                Err(e) => panic!("[Subnet.describe] ERROR\n{}", e),
            },
        }
    }

    pub async fn delete(client: &Client, subnet_id: &str) -> Result<(), Error> {
        match client
            .delete_subnet()
            .set_subnet_id(Some(subnet_id.to_string()))
            .send()
            .await
        {
            Ok(response) => {
                println!("[Subnet.delete] success {}", subnet_id);
                Ok(())
            }
            Err(e) => {
                panic!("[Subnet.delete] ERROR {}", e);
            }
        }
    }
}

// Security Groups

pub struct SecurityGroup {}
impl SecurityGroup {
    pub async fn create(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        let vpc_id = sc.vpc_id.as_ref().unwrap();
        let tag_specifications = create_tag_spec(sc, types::ResourceType::SecurityGroup);
        let ssh_cidr_block = sc.ssh_cidr_block.as_ref().unwrap();
        println!("[SecurityGroup.create] vpc_id {:?}", vpc_id);
        println!("[SecurityGroup.create] tags {:?}", tag_specifications);
        println!("[SecurityGroup.create] ssh cidr {:?}", ssh_cidr_block);

        let response = client
            .create_security_group()
            .vpc_id(vpc_id.clone())
            .group_name("allow-ssh")
            .description("Allow SSH inbound traffic")
            .tag_specifications(tag_specifications)
            .send()
            .await?;

        let sg_id = response.group_id.unwrap();

        println!("[SecurityGroup.create] success {:?}", sg_id);

        // Add ingress rule to allow SSH
        client
            .authorize_security_group_ingress()
            .group_id(&sg_id)
            .set_ip_permissions(Some(vec![
                types::IpPermission::builder()
                    .ip_protocol("tcp")
                    .from_port(22)
                    .to_port(22)
                    .ip_ranges(
                        types::IpRange::builder()
                            .cidr_ip(ssh_cidr_block.to_string())
                            .build(),
                    )
                    .build(),
            ]))
            .send()
            .await?;

        println!("[SecurityGroup.create] ingress");

        // Add egress rule to allow all outbound traffic
        client
            .authorize_security_group_egress()
            .group_id(&sg_id)
            .set_ip_permissions(Some(vec![
                types::IpPermission::builder()
                    .ip_protocol("tcp")
                    .from_port(0)
                    .to_port(0)
                    .ip_ranges(types::IpRange::builder().cidr_ip("0.0.0.0/0").build())
                    .build(),
            ]))
            .send()
            .await?;

        println!("[SecurityGroup.create] egress");

        Ok(sg_id.clone())
    }

    pub async fn describe(
        client: &Client,
        matcher: ResourceMatcher,
    ) -> Result<Vec<types::SecurityGroup>, Error> {
        let r = client.describe_security_groups();
        let request = match matcher {
            ResourceMatcher::Id(sg_ids) => match sg_ids.len() {
                0 => None,
                _ => Some(r.set_group_ids(Some(sg_ids.clone())).send()),
            },
            ResourceMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag)).send()),
        };

        match request {
            None => Ok(Vec::new()),
            Some(response) => match response.await {
                Ok(response) => {
                    let sgs = response.security_groups.clone().unwrap();
                    match sgs.len() {
                        0 => Ok(Vec::new()),
                        _ => Ok(sgs.clone()),
                    }
                }
                Err(e) => panic!("[VPC.describe] ERROR\n{}", e),
            },
        }
    }

    pub async fn delete(client: &Client, security_group_id: &str) -> Result<(), Error> {
        match client
            .delete_security_group()
            .set_group_id(Some(security_group_id.to_string()))
            .send()
            .await
        {
            Ok(response) => {
                println!("[SecurityGroup.delete] success {}", security_group_id);
                Ok(())
            }
            Err(e) => {
                panic!("[SecurityGroup.delete] ERROR {}", e);
            }
        }
    }
}

// Key Pairs

pub struct SSHKey {}
impl SSHKey {
    pub async fn import(
        client: &Client,
        sc: &SwarmConfig,
        key_name: &str,
    ) -> Result<String, Error> {
        println!("[SSHKey.import] name {}", key_name);
        println!("[SSHKey.import] key_file {}", sc.key_file.clone());

        let tag_specifications = create_tag_spec(sc, types::ResourceType::KeyPair);

        let key_path = PathBuf::from(sc.key_file.clone());
        println!("[SSHKey.import] key_file {:?}", fs::canonicalize(&key_path));

        let key_material = match std::fs::read_to_string(sc.key_file.clone()) {
            Ok(key_material) => key_material,
            Err(e) => panic!("[SSHKey.import] read_to_string\n{}", e),
        };
        println!("[SSHKey.import] key material loaded");

        let key_blob = aws_sdk_ec2::primitives::Blob::new(key_material);

        let response = match client
            .import_key_pair()
            .key_name(key_name)
            .public_key_material(key_blob)
            .tag_specifications(tag_specifications)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => panic!("[SSHKey.import] ERROR import call\n{}", e),
        };

        let key_id = response.key_pair_id.unwrap();
        println!("[SSHKey.import] success {:?}", key_id);
        Ok(key_id)
    }

    pub async fn describe(
        client: &Client,
        key_name: &str,
    ) -> Result<Vec<types::KeyPairInfo>, Error> {
        println!("[SSHKey.describe] key_name {}", key_name);

        match client.describe_key_pairs().key_names(key_name).send().await {
            Ok(response) => {
                let key_pairs = response.key_pairs.unwrap();
                println!("[SSHKey.describe] success {:?}", key_pairs.len());
                Ok(key_pairs)
            }
            Err(e) => {
                println!("[SSHKey.describe] no key found");
                Ok(Vec::new())
            }
        }
    }

    pub async fn delete(client: &Client, key_name: &str) -> Result<(), Error> {
        match client
            .delete_key_pair()
            .set_key_name(Some(key_name.to_string()))
            .send()
            .await
        {
            Ok(response) => {
                println!("[SSHKey.delete] success {}", key_name);
                Ok(())
            }
            Err(e) => {
                panic!("[SSHKey.delete] ERROR {}", e);
            }
        }
    }
}

/// A `Bee` is an instance id and its public IP address
#[derive(Debug, Clone)]
pub struct Bee {
    pub id: String,
    pub ip: Option<String>,
}

/// Beez can be loaded via id list or a resource tag
pub enum BeeMatcher {
    Ids(Vec<Bee>),
    Tagged(String),
}

// Instances

pub struct Instances {}
impl Instances {
    pub async fn create(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
        count_delta: Option<i32>,
    ) -> Result<Vec<Bee>, Error> {
        println!("[Instances.create]");
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Instance);

        let new_beez = match count_delta {
            Some(cd) => cd,
            None => sc.num_beez,
        };

        let response = match client
            .run_instances()
            .instance_type(types::InstanceType::T2Micro)
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
            Err(e) => panic!("[Instances.create] ERROR create {:?}", e),
        };

        if response.instances().is_empty() {
            panic!("[Instances.create] ERROR no instances created");
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

    pub async fn describe(client: &Client, matcher: BeeMatcher) -> Result<Vec<Bee>, Error> {
        println!("[Instances.describe]");

        let r = client.describe_instances();
        let request = match matcher {
            BeeMatcher::Ids(ids) => {
                let ids_vec = ids.iter().map(|b| b.id.clone()).collect::<Vec<String>>();
                match ids.len() {
                    0 => None,
                    _ => Some(r.set_instance_ids(Some(ids_vec)).send()),
                }
            }
            BeeMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag)).send()),
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
                                    types::InstanceStateName::Running
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

    pub async fn delete(
        client: &Client,
        sc: &SwarmConfig,
        matcher: &BeeMatcher,
    ) -> Result<(), Error> {
        // multiple branches below need this
        let terminate_beez = |beez: Vec<Bee>| {
            let bee_ids = beez.iter().map(|b| b.id.clone()).collect::<Vec<String>>();
            let r = client.terminate_instances();
            Some(r.set_instance_ids(Some(bee_ids)).send())
        };

        let request = match matcher {
            // terminate list of ids
            BeeMatcher::Ids(beez) => match beez.len() {
                0 => None,
                _ => terminate_beez(beez.clone()),
            },
            // convert tag into list of ids, then terminate
            BeeMatcher::Tagged(tag) => {
                let m = BeeMatcher::Tagged(sc.tag_name.clone());
                match Instances::describe(client, m).await {
                    Ok(beez) => terminate_beez(beez),
                    _ => None,
                }
            }
        };

        match request {
            None => Ok(()),
            Some(response) => match response.await {
                Ok(response) => {
                    println!("[Instances.delete {:?}", response.terminating_instances);
                    Ok(())
                }
                Err(e) => {
                    panic!("Error terminating instances: {}", e);
                }
            },
        }
    }

    pub async fn wait_for_running(client: &Client, beez: Vec<Bee>) -> Result<Vec<Bee>, Error> {
        println!("[Instances.wait_for_running]");
        loop {
            let m = BeeMatcher::Ids(beez.clone());
            let running_beez = Instances::describe(client, m).await.unwrap();

            // return Ok when counts match
            let delta = beez.len() - running_beez.len();
            if delta == 0 {
                return Ok(running_beez.clone());
            }
            println!(
                "[Instances.wait_for_running] waiting 15 seconds for {} beez",
                delta
            );
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    }
}
