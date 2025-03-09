use aws_config::{ConfigLoader, meta::region::RegionProviderChain};
use aws_sdk_ec2::{
    Client, Error,
    client::Waiters,
    error::SdkError,
    operation::{create_vpc::CreateVpcError, delete_vpc::builders::DeleteVpcFluentBuilder},
    types::{
        self,
        builders::{
            IpPermissionBuilder, NetworkInterfaceBuilder, TagSpecificationBuilder, VpcBuilder,
        },
    },
};
use clap::builder::OsStr;
use std::{collections::HashMap, fs, ops::Deref, path::PathBuf, ptr::read, time::Duration};

use crate::{config::SwarmConfig, scenarios::AWSNetwork};

pub async fn mk_client() -> Result<Client, Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2024_03_28()).await;
    Ok(Client::new(&config))
}

// Tags

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

fn create_tag_filter(tag: &str) -> types::Filter {
    types::Filter::builder()
        .name("tag:Name")
        .values(tag)
        .build()
}

/// Beez can be loaded via id list or a resource tag
#[derive(Debug, Clone)]
pub enum ResourceMatcher {
    Id(Vec<String>),
    Tagged(String),
}

// VPCs

pub struct VPC {}
impl VPC {
    pub async fn create(client: &Client, sc: &SwarmConfig) -> Result<types::Vpc, Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Vpc);
        println!("[VPC.create]");
        println!("[VPC.create] tags: {:?}", tag_specifications);

        let request = client
            .create_vpc()
            .cidr_block("10.0.0.0/16")
            .tag_specifications(tag_specifications)
            .send();

        match request.await {
            Ok(response) => match response.vpc {
                Some(vpc) => Ok(vpc.clone()),
                None => unimplemented!(),
            },
            Err(e) => panic!("[VPC.create] ERROR\n{}", e),
        }
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
            ResourceMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag.clone())).send()),
        };

        match request {
            None => Ok(Vec::new()),
            Some(request) => match request.await {
                Ok(response) => match response.vpcs {
                    Some(vpcs) => match vpcs.len() {
                        0 => Ok(Vec::new()),
                        _ => Ok(vpcs.clone()),
                    },
                    None => unimplemented!(),
                },
                Err(e) => panic!("[VPC.describe] ERROR\n{}", e),
            },
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Error> {
        async fn terminate_ids(client: &Client, vpc_ids: Vec<String>) -> Result<(), Error> {
            match vpc_ids.len() {
                0 => Ok(()),
                _ => {
                    let r = client.delete_vpc();
                    match vpc_ids.first() {
                        Some(vpc_id) => match r.set_vpc_id(Some(vpc_id.clone())).send().await {
                            Ok(_) => Ok(()),
                            Err(e) => panic!("[OH NO] ERROR\n{}", e),
                        },
                        None => Ok(()),
                    }
                }
            }
        }

        match matcher {
            ResourceMatcher::Id(vpc_ids) => match vpc_ids.len() {
                0 => Ok(()),
                _ => terminate_ids(client, vpc_ids.clone()).await,
            },
            m @ ResourceMatcher::Tagged(_) => match VPC::describe(client, m.clone()).await {
                Ok(vpcs) => {
                    let vpc_ids = vpcs
                        .iter()
                        .filter_map(|b| b.vpc_id.clone())
                        .collect::<Vec<String>>();
                    match vpc_ids.len() {
                        0 => Ok(()),
                        _ => terminate_ids(client, vpc_ids.clone()).await,
                    }
                }
                Err(e) => unimplemented!(),
            },
        }
    }
}

// Subnets

pub struct Subnet {}
impl Subnet {
    pub async fn create(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<types::Subnet, Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Subnet);
        println!("[Subnet.create] vpc_id {}", vpc_id);
        println!("[Subnet.create] tags: {:?}", tag_specifications);

        let response = client
            .create_subnet()
            .vpc_id(vpc_id)
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
        let r = client.describe_subnets();
        let request = match matcher {
            ResourceMatcher::Id(subnet_ids) => match subnet_ids.len() {
                0 => None,
                _ => Some(r.set_subnet_ids(Some(subnet_ids.clone())).send()),
            },
            ResourceMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag.clone())).send()),
        };

        match request {
            None => Ok(Vec::new()),
            Some(request) => match request.await {
                Ok(response) => match response.subnets {
                    Some(subnets) => match subnets.len() {
                        0 => Ok(Vec::new()),
                        _ => Ok(subnets.clone()),
                    },
                    None => unimplemented!(),
                },
                Err(e) => panic!("[Subnet.describe] ERROR\n{}", e),
            },
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Error> {
        async fn terminate_ids(client: &Client, subnet_ids: Vec<String>) -> Result<(), Error> {
            match subnet_ids.len() {
                0 => Ok(()),
                _ => {
                    let r = client.delete_subnet();
                    match subnet_ids.first() {
                        Some(subnet_id) => {
                            match r.set_subnet_id(Some(subnet_id.clone())).send().await {
                                Ok(_) => Ok(()),
                                Err(e) => unimplemented!(),
                            }
                        }
                        None => Ok(()),
                    }
                }
            }
        }

        match matcher {
            ResourceMatcher::Id(subnet_ids) => match subnet_ids.len() {
                0 => Ok(()),
                _ => terminate_ids(client, subnet_ids.clone()).await,
            },
            m @ ResourceMatcher::Tagged(_) => match Subnet::describe(client, m.clone()).await {
                Ok(subnets) => {
                    let subnet_ids = subnets
                        .iter()
                        .filter_map(|b| b.subnet_id.clone())
                        .collect::<Vec<String>>();
                    match subnet_ids.len() {
                        0 => Ok(()),
                        _ => terminate_ids(client, subnet_ids).await,
                    }
                }
                Err(e) => unimplemented!(),
            },
        }
    }
}

// Security Groups

pub struct SecurityGroup {}
impl SecurityGroup {
    pub async fn create(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
        subnet_id: &str,
    ) -> Result<String, Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::SecurityGroup);
        let ssh_cidr_block = sc.ssh_cidr_block.clone().unwrap();
        println!("[SecurityGroup.create] tags {:?}", tag_specifications);
        println!("[SecurityGroup.create] vpc_id {:?}", vpc_id);
        println!("[SecurityGroup.create] ssh cidr {:?}", ssh_cidr_block);

        let response = client
            .create_security_group()
            .vpc_id(vpc_id)
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
            ResourceMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag.clone())).send()),
        };

        match request {
            None => Ok(Vec::new()),
            Some(request) => match request.await {
                Ok(response) => match response.security_groups {
                    Some(sgs) => match sgs.len() {
                        0 => Ok(Vec::new()),
                        _ => Ok(sgs.clone()),
                    },
                    None => unimplemented!(),
                },
                Err(e) => panic!("[VPC.describe] ERROR\n{}", e),
            },
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Error> {
        async fn terminate_ids(client: &Client, sg_ids: Vec<String>) -> Result<(), Error> {
            match sg_ids.len() {
                0 => Ok(()),
                _ => {
                    let r = client.delete_security_group();
                    match sg_ids.first() {
                        Some(sg_id) => match r.set_group_id(Some(sg_id.clone())).send().await {
                            Ok(_) => Ok(()),
                            Err(e) => unimplemented!(),
                        },
                        None => Ok(()),
                    }
                }
            }
        }

        match matcher {
            ResourceMatcher::Id(sg_ids) => match sg_ids.len() {
                0 => Ok(()),
                _ => terminate_ids(client, sg_ids.clone()).await,
            },
            m @ ResourceMatcher::Tagged(_) => {
                match SecurityGroup::describe(client, m.clone()).await {
                    Ok(sgs) => {
                        let sg_ids = sgs
                            .iter()
                            .filter_map(|b| b.group_id.clone())
                            .collect::<Vec<String>>();
                        match sg_ids.len() {
                            0 => Ok(()),
                            _ => terminate_ids(client, sg_ids).await,
                        }
                    }
                    Err(e) => unimplemented!(),
                }
            }
        }
    }
}

// Key Pairs

#[derive(Debug, Clone)]
pub enum SSHKeyMatcher {
    Id(String),
    Name(String),
}

pub struct SSHKey {}
impl SSHKey {
    pub async fn import(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        println!("[SSHKey.import] key_file {:?}", sc.public_key_file.clone());

        let Some(pk_file) = sc.public_key_file.clone() else {
            unimplemented!()
        };

        let tag_specifications = create_tag_spec(sc, types::ResourceType::KeyPair);

        let pk_path = PathBuf::from(&pk_file);
        println!("[SSHKey.import] key_file {:?}", fs::canonicalize(&pk_path));

        let key_material = match std::fs::read_to_string(&pk_file) {
            Ok(key_material) => key_material,
            Err(e) => panic!("[SSHKey.import] read_to_string\n{}", e),
        };
        println!("[SSHKey.import] key material loaded");

        let key_blob = aws_sdk_ec2::primitives::Blob::new(key_material);

        match client
            .import_key_pair()
            .key_name(sc.tag_name.clone())
            .public_key_material(key_blob)
            .tag_specifications(tag_specifications)
            .send()
            .await
        {
            Ok(response) => Ok(response.key_pair_id.clone().unwrap()),
            Err(e) => panic!("[SSHKey.import] ERROR import call\n{}", e),
        }
    }

    pub async fn describe(
        client: &Client,
        matcher: SSHKeyMatcher,
    ) -> Result<Vec<types::KeyPairInfo>, Error> {
        let r = client.describe_key_pairs();
        match matcher.clone() {
            SSHKeyMatcher::Id(key_id) => {
                let id_param = vec![key_id.clone()];
                let r = r.set_key_pair_ids(Some(id_param)).send();
                match r.await {
                    Ok(response) => match response.key_pairs {
                        Some(key_pairs) => match key_pairs.len() {
                            0 => Ok(Vec::new()),
                            _ => Ok(key_pairs.clone()),
                        },
                        None => Ok(Vec::new()),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            SSHKeyMatcher::Name(key_name) => {
                let name_param = vec![key_name.clone()];
                let r = r.set_key_names(Some(name_param)).send();
                match r.await {
                    Ok(response) => match response.key_pairs {
                        Some(key_pairs) => match key_pairs.len() {
                            0 => Ok(Vec::new()),
                            _ => Ok(key_pairs.clone()),
                        },
                        None => Ok(Vec::new()),
                    },
                    // this happens when the key name isn't found
                    Err(e) => Ok(Vec::new()),
                }
            }
        }
    }

    pub async fn delete(client: &Client, matcher: SSHKeyMatcher) -> Result<(), Error> {
        let r = client.delete_key_pair();
        let result = match matcher {
            SSHKeyMatcher::Id(key_id) => r.set_key_pair_id(Some(key_id)),
            SSHKeyMatcher::Name(key_name) => r.set_key_name(Some(key_name.to_string())),
        };
        match result.send().await {
            Ok(_) => Ok(()),
            Err(e) => unimplemented!(),
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
#[derive(Debug, Clone)]
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
            .key_name(sc.tag_name.clone())
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

    pub async fn describe(
        client: &Client,
        matcher: BeeMatcher,
        state: types::InstanceStateName,
    ) -> Result<Vec<Bee>, Error> {
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
            BeeMatcher::Tagged(tag) => Some(r.filters(create_tag_filter(&tag.clone())).send()),
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
                            .filter(|&i| match i.clone().state.unwrap().name {
                                Some(name) => name.eq(&state.clone()),
                                None => false,
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
    ) -> Result<Vec<Bee>, Error> {
        // multiple branches below need this
        async fn terminate_beez(client: &Client, beez: Vec<Bee>) -> Result<Vec<Bee>, Error> {
            let bee_ids = beez.iter().map(|b| b.id.clone()).collect::<Vec<String>>();
            match bee_ids.len() {
                0 => Ok(Vec::new()),
                _ => {
                    let r = client.terminate_instances();
                    match r.set_instance_ids(Some(bee_ids.clone())).send().await {
                        Ok(_) => Ok(beez.clone()),
                        Err(e) => unimplemented!(),
                    }
                }
            }
        }

        match matcher {
            // terminate list of ids
            BeeMatcher::Ids(beez) => match beez.len() {
                0 => Ok(Vec::new()),
                _ => terminate_beez(client, beez.clone()).await,
            },
            // convert tag into list of ids, then terminate
            m @ BeeMatcher::Tagged(_) => {
                match Instances::describe(client, m.clone(), types::InstanceStateName::Running)
                    .await
                {
                    Ok(beez) => terminate_beez(client, beez.clone()).await,
                    _ => Ok(Vec::new()),
                }
            }
        }
    }

    pub async fn wait(
        client: &Client,
        beez: Vec<Bee>,
        state: types::InstanceStateName,
    ) -> Result<Vec<Bee>, Error> {
        let mut delta = beez.len();
        let wait_seconds = 15;
        loop {
            println!(
                "[Instances] waiting {} seconds for {} beez",
                wait_seconds, delta
            );
            tokio::time::sleep(Duration::from_secs(wait_seconds)).await;

            let m = BeeMatcher::Ids(beez.clone());
            match Instances::describe(client, m.clone(), state.clone()).await {
                Ok(running_beez) => match beez.len() - running_beez.clone().len() {
                    0 => return Ok(running_beez.clone()),
                    d => delta = d,
                },
                Err(e) => unimplemented!(),
            }
        }
    }
}
