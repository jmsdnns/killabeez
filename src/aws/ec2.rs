use std::{collections::HashMap, fmt, fs, ops::Deref, path::PathBuf, ptr::read, time::Duration};

use aws_config::ConfigLoader;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_ec2::client::Waiters;
use aws_sdk_ec2::error::SdkError;
use aws_sdk_ec2::operation::{
    create_vpc::CreateVpcError, delete_vpc::builders::DeleteVpcFluentBuilder,
    describe_instances::DescribeInstancesError, run_instances::RunInstancesError,
    terminate_instances::TerminateInstancesError,
};
use aws_sdk_ec2::types;
use aws_sdk_ec2::types::builders::{
    IpPermissionBuilder, NetworkInterfaceBuilder, TagSpecificationBuilder, VpcBuilder,
};
use aws_sdk_ec2::{Client, Error};
use clap::builder::OsStr;

use crate::aws::errors::Ec2Error;
use crate::config::SwarmConfig;
use crate::scenarios::AWSNetwork;

// hard coded for now
const CIDR_VPC: &str = "10.0.0.0/16";
const CIDR_SUBNET: &str = "10.0.1.0/24";
const CIDR_GATEWAY: &str = "0.0.0.0/0";

pub async fn mk_client() -> Client {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2024_03_28()).await;
    Client::new(&config)
}

pub async fn hold_on(duration: u64) {
    // give it a moment
    tokio::time::sleep(Duration::from_secs(duration)).await;
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
    pub async fn create(client: &Client, sc: &SwarmConfig) -> Result<types::Vpc, Ec2Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Vpc);
        println!("[VPC.create]");
        println!("[VPC.create] tags: {:?}", tag_specifications);

        Ok(client
            .create_vpc()
            .cidr_block(CIDR_VPC)
            .tag_specifications(tag_specifications)
            .send()
            .await?
            .vpc
            .unwrap())
    }

    pub async fn describe(
        client: &Client,
        matcher: ResourceMatcher,
    ) -> Result<Vec<types::Vpc>, Ec2Error> {
        match matcher {
            ResourceMatcher::Id(vpc_ids) => match vpc_ids.len() {
                0 => Ok(Vec::new()),
                _ => Ok(client
                    .describe_vpcs()
                    .set_vpc_ids(Some(vpc_ids.clone()))
                    .send()
                    .await?
                    .vpcs
                    .unwrap()),
            },
            ResourceMatcher::Tagged(tag) => Ok(client
                .describe_vpcs()
                .filters(create_tag_filter(&tag.clone()))
                .send()
                .await?
                .vpcs
                .unwrap()),
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Ec2Error> {
        async fn terminate_ids(client: &Client, vpc_ids: Vec<String>) -> Result<(), Ec2Error> {
            let vpc_id = vpc_ids.first().unwrap().to_owned();
            client.delete_vpc().set_vpc_id(Some(vpc_id)).send().await?;
            Ok(())
        }

        match matcher {
            ResourceMatcher::Id(vpc_ids) => terminate_ids(client, vpc_ids.clone()).await,
            m @ ResourceMatcher::Tagged(_) => {
                let vpc_ids = VPC::describe(client, m.clone())
                    .await?
                    .iter()
                    .filter_map(|b| b.vpc_id.clone())
                    .collect::<Vec<String>>();
                terminate_ids(client, vpc_ids).await
            }
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
    ) -> Result<types::Subnet, Ec2Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Subnet);
        println!("[Subnet.create] vpc_id {}", vpc_id);
        println!("[Subnet.create] tags: {:?}", tag_specifications);

        let subnet = client
            .create_subnet()
            .vpc_id(vpc_id)
            .cidr_block(CIDR_SUBNET)
            .tag_specifications(tag_specifications)
            .send()
            .await?
            .subnet
            .unwrap();

        let subnet_id = subnet.subnet_id().unwrap();
        println!("[Subnet.create] success {:?}", subnet_id);

        client
            .modify_subnet_attribute()
            .subnet_id(subnet_id)
            .map_public_ip_on_launch(types::AttributeBooleanValue::builder().value(true).build())
            .send()
            .await?;
        println!("[Subnet.create] maps public ip on launch");

        Ok(subnet.to_owned())
    }

    pub async fn describe(
        client: &Client,
        matcher: ResourceMatcher,
    ) -> Result<Vec<types::Subnet>, Ec2Error> {
        match matcher {
            ResourceMatcher::Id(subnet_ids) => match subnet_ids.len() {
                0 => Ok(Vec::new()),
                _ => Ok(client
                    .describe_subnets()
                    .set_subnet_ids(Some(subnet_ids.clone()))
                    .send()
                    .await?
                    .subnets
                    .unwrap()),
            },
            ResourceMatcher::Tagged(tag) => Ok(client
                .describe_subnets()
                .filters(create_tag_filter(&tag.clone()))
                .send()
                .await?
                .subnets
                .unwrap()),
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Ec2Error> {
        async fn terminate_ids(client: &Client, subnet_ids: Vec<String>) -> Result<(), Ec2Error> {
            let subnet_id = subnet_ids.first().unwrap().to_owned();
            client
                .delete_subnet()
                .set_subnet_id(Some(subnet_id))
                .send()
                .await?;
            Ok(())
        }

        match matcher {
            ResourceMatcher::Id(subnet_ids) => terminate_ids(client, subnet_ids.clone()).await,
            m @ ResourceMatcher::Tagged(_) => {
                let subnet_ids = Subnet::describe(client, m.clone())
                    .await?
                    .iter()
                    .filter_map(|b| b.subnet_id.clone())
                    .collect::<Vec<String>>();
                terminate_ids(client, subnet_ids).await
            }
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
    ) -> Result<String, Ec2Error> {
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
    ) -> Result<Vec<types::SecurityGroup>, Ec2Error> {
        let r = client.describe_security_groups();
        match matcher {
            ResourceMatcher::Id(sg_ids) => match sg_ids.len() {
                0 => Ok(Vec::new()),
                _ => Ok(client
                    .describe_security_groups()
                    .set_group_ids(Some(sg_ids.clone()))
                    .send()
                    .await?
                    .security_groups
                    .unwrap()),
            },
            ResourceMatcher::Tagged(tag) => Ok(client
                .describe_security_groups()
                .filters(create_tag_filter(&tag.clone()))
                .send()
                .await?
                .security_groups
                .unwrap()),
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Ec2Error> {
        async fn terminate_ids(client: &Client, sg_ids: Vec<String>) -> Result<(), Ec2Error> {
            let sg_id = sg_ids.first().unwrap().to_owned();
            client
                .delete_security_group()
                .set_group_id(Some(sg_id))
                .send()
                .await?;
            Ok(())
        }

        match matcher {
            ResourceMatcher::Id(sg_ids) => terminate_ids(client, sg_ids.clone()).await,
            m @ ResourceMatcher::Tagged(_) => {
                let sg_ids = SecurityGroup::describe(client, m.clone())
                    .await?
                    .iter()
                    .filter_map(|b| b.group_id.clone())
                    .collect::<Vec<String>>();
                terminate_ids(client, sg_ids).await
            }
        }
    }
}

// Internet Gateway

pub struct InternetGateway {}
impl InternetGateway {
    pub async fn create(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<types::InternetGateway, Ec2Error> {
        let tag_specifications = create_tag_spec(sc, types::ResourceType::InternetGateway);
        println!("[Gateway.create] vpc_id {}", vpc_id);
        println!("[Gateway.create] tags: {:?}", tag_specifications);

        let igw = client
            .create_internet_gateway()
            .tag_specifications(tag_specifications)
            .send()
            .await?
            .internet_gateway
            .unwrap();

        match InternetGateway::attach(client, igw.clone(), vpc_id).await {
            Ok(()) => Ok(igw),
            Err(e) => panic!("[Gateway.create] ERROR {}", e),
        }
    }

    pub async fn describe(
        client: &Client,
        matcher: ResourceMatcher,
    ) -> Result<Vec<types::InternetGateway>, Ec2Error> {
        match matcher {
            ResourceMatcher::Id(igw_ids) => match igw_ids.len() {
                0 => Ok(Vec::new()),
                _ => Ok(client
                    .describe_internet_gateways()
                    .set_internet_gateway_ids(Some(igw_ids.clone()))
                    .send()
                    .await?
                    .internet_gateways
                    .unwrap()),
            },
            ResourceMatcher::Tagged(tag) => Ok(client
                .describe_internet_gateways()
                .filters(create_tag_filter(&tag.clone()))
                .send()
                .await?
                .internet_gateways
                .unwrap()),
        }
    }

    pub async fn delete(client: &Client, matcher: ResourceMatcher) -> Result<(), Ec2Error> {
        async fn terminate_ids(client: &Client, igw_ids: Vec<String>) -> Result<(), Ec2Error> {
            let igw_id = igw_ids.first().unwrap().to_owned();
            match InternetGateway::detach(client, &igw_id).await {
                Ok(()) => (),
                Err(e) => panic!("[Gateway.create] ERROR {}", e),
            };
            client
                .delete_internet_gateway()
                .set_internet_gateway_id(Some(igw_id.clone()))
                .send()
                .await?;
            Ok(())
        }

        match matcher {
            ResourceMatcher::Id(igw_ids) => match igw_ids.len() {
                0 => Ok(()),
                _ => terminate_ids(client, igw_ids.clone()).await,
            },
            m @ ResourceMatcher::Tagged(_) => {
                let igw_ids = InternetGateway::describe(client, m.clone())
                    .await?
                    .iter()
                    .filter_map(|b| b.internet_gateway_id.clone())
                    .collect::<Vec<String>>();
                terminate_ids(client, igw_ids).await
            }
        }
    }

    pub async fn attached_vpc_id(client: &Client, igw_id: &str) -> Result<Option<String>, Error> {
        match InternetGateway::describe(client, ResourceMatcher::Id(vec![igw_id.to_string()])).await
        {
            Ok(igws) => match igws.len() {
                1 => match igws.first().unwrap().attachments().first() {
                    Some(att) => Ok(Some(att.vpc_id.clone().unwrap())),
                    _ => Ok(None),
                },
                _ => unimplemented!(),
            },
            Err(e) => panic!("OHHHHH NO {}", e),
        }
    }

    pub async fn attach(
        client: &Client,
        igw: types::InternetGateway,
        vpc_id: &str,
    ) -> Result<(), Error> {
        let igw_id = match igw.internet_gateway_id.clone() {
            Some(igw_id) => igw_id,
            None => panic!("No IGW found"),
        };

        // attach internet gateway to vpc
        match client
            .attach_internet_gateway()
            .set_internet_gateway_id(Some(igw_id.to_string()))
            .set_vpc_id(Some(vpc_id.to_string()))
            .send()
            .await
        {
            Ok(_) => (),
            Err(e) => panic!("[Gateway.create ERROR {}", e),
        };

        // give it a moment
        hold_on(5).await;

        // load vpc route tables
        let rt_id = match client
            .describe_route_tables()
            .filters(
                types::Filter::builder()
                    .name("vpc-id")
                    .values(vpc_id)
                    .build(),
            )
            .send()
            .await
        {
            Ok(request) => {
                match request
                    .route_tables
                    .unwrap()
                    .first()
                    .unwrap()
                    .route_table_id
                    .clone()
                {
                    Some(rt_id) => rt_id,
                    None => unimplemented!(),
                }
            }
            Err(e) => panic!("[attach] {}", e),
        };

        println!("[Gateway.attach] route id {}", rt_id);

        // create routes for public access
        let request = client
            .create_route()
            .set_route_table_id(Some(rt_id))
            .destination_cidr_block(CIDR_GATEWAY)
            .set_gateway_id(Some(igw_id))
            .send()
            .await;
        match request {
            Ok(_) => Ok(()),
            Err(e) => panic!("[Gateway.attach] ERROR {}", e),
        }
    }

    pub async fn detach(client: &Client, igw_id: &str) -> Result<(), Error> {
        let vpc_id = match InternetGateway::attached_vpc_id(client, igw_id).await {
            Ok(Some(igw_id)) => igw_id,
            Ok(None) => return Ok(()),
            Err(e) => panic!("OHHHHH NO {}", e),
        };

        // detach internet gateway from vpc
        match client
            .detach_internet_gateway()
            .set_internet_gateway_id(Some(igw_id.to_string()))
            .set_vpc_id(Some(vpc_id.to_string()))
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => panic!("[Gateway.detach ERROR {}", e),
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
    pub async fn import(client: &Client, sc: &SwarmConfig) -> Result<String, Ec2Error> {
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

        Ok(client
            .import_key_pair()
            .key_name(sc.tag_name.clone())
            .public_key_material(key_blob)
            .tag_specifications(tag_specifications)
            .send()
            .await?
            .key_pair_id
            .unwrap())
    }

    pub async fn describe(
        client: &Client,
        matcher: SSHKeyMatcher,
    ) -> Result<Vec<types::KeyPairInfo>, Ec2Error> {
        match matcher.clone() {
            SSHKeyMatcher::Id(key_id) => Ok(client
                .describe_key_pairs()
                .set_key_pair_ids(Some(vec![key_id.clone()]))
                .send()
                .await?
                .key_pairs
                .unwrap()),
            SSHKeyMatcher::Name(key_name) => Ok(client
                .describe_key_pairs()
                .set_key_names(Some(vec![key_name.clone()]))
                .send()
                .await?
                .key_pairs
                .unwrap()),
        }
    }

    pub async fn delete(client: &Client, matcher: SSHKeyMatcher) -> Result<(), Ec2Error> {
        let mut r = client.delete_key_pair();
        match matcher {
            SSHKeyMatcher::Id(key_id) => r.clone().set_key_pair_id(Some(key_id)),
            SSHKeyMatcher::Name(key_name) => r.clone().set_key_name(Some(key_name.to_string())),
        };
        r.send().await?;
        Ok(())
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
    ) -> Result<Vec<Bee>, Ec2Error> {
        println!("[Instances.create]");
        let tag_specifications = create_tag_spec(sc, types::ResourceType::Instance);

        let new_beez = match count_delta {
            Some(cd) => cd,
            None => sc.num_beez,
        };

        let response = client
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
            .await?;

        if response.instances().is_empty() {
            panic!("[Instances.create] ERROR no instances created");
        }

        let instances = response
            .instances()
            .iter()
            .map(|i| Bee {
                id: i.instance_id.clone().unwrap(),
                ip: i.public_ip_address.clone(),
            })
            .collect::<Vec<Bee>>();

        Ok(instances)
    }

    pub async fn describe(
        client: &Client,
        matcher: BeeMatcher,
        state: types::InstanceStateName,
    ) -> Result<Vec<Bee>, Ec2Error> {
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
    ) -> Result<Vec<Bee>, Ec2Error> {
        // multiple branches below need this
        async fn terminate_beez(client: &Client, beez: Vec<Bee>) -> Result<Vec<Bee>, Ec2Error> {
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
            hold_on(wait_seconds).await;

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
