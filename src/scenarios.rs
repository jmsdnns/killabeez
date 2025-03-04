use aws_sdk_ec2::waiters::security_group_exists;
use aws_sdk_ec2::{Client, Error};

use crate::aws;
use crate::aws::ec2::{
    Bee, BeeMatcher, Instances, ResourceMatcher, SSHKey, SecurityGroup, Subnet, VPC,
};
use crate::config::SwarmConfig;

#[derive(Debug, Clone)]
pub struct AWSNetwork {
    pub vpc_id: String,
    pub subnet_id: String,
    pub security_group_id: String,
}

impl AWSNetwork {
    pub async fn load_network(client: &Client, sc: &SwarmConfig) -> Result<Self, Error> {
        println!("[load_network]");

        let vpc_id = match AWSNetwork::load_vpc(client, sc).await {
            Ok(vpc_id) => vpc_id,
            Err(e) => panic!("[load_network] ERROR load_vpc\n{}", e),
        };
        let subnet_id = match AWSNetwork::load_subnet(client, sc).await {
            Ok(subnet_id) => subnet_id,
            Err(e) => panic!("[load_network] ERROR load_subnet\n{}", e),
        };
        let security_group_id = match AWSNetwork::load_security_group(client, sc).await {
            Ok(security_group_id) => security_group_id,
            Err(e) => panic!("[load_network] ERROR load_security_group\n{}", e),
        };

        Ok(AWSNetwork {
            vpc_id: vpc_id.clone(),
            subnet_id: subnet_id.clone(),
            security_group_id: security_group_id.clone(),
        })
    }

    pub async fn load_vpc(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        println!("[load_vpc]");

        let existing_vpc_id = match sc.vpc_id.clone() {
            Some(sc_vpc_id) => {
                let m = &ResourceMatcher::Id(vec![sc_vpc_id.clone()]);
                let vpcs = match VPC::describe(client, m).await {
                    Ok(vpcs) => vpcs,
                    Err(e) => panic!("[load_vpc] ERROR load_vpc {:?}", e),
                };
                if vpcs.is_empty() {
                    None
                } else {
                    Some(sc_vpc_id)
                }
            }
            None => None,
        };

        println!("[load vpc] sc.vpc_id {:?}", existing_vpc_id.clone());

        let final_vpc_id = match existing_vpc_id {
            None => {
                let Ok(vpc) = VPC::create(client, sc).await else {
                    panic!("[load_vpc] ERROR create vpc");
                };
                Some(vpc.vpc_id.unwrap().clone())
            }
            Some(vpc_id) => Some(vpc_id),
        };
        println!(
            "[load_vpc] final_vpc_id {:?}",
            final_vpc_id.as_ref().clone()
        );
        Ok(final_vpc_id.unwrap().clone())
    }

    pub async fn load_subnet(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        println!("[load_subnet]");

        let existing_subnet_id = match sc.subnet_id.clone() {
            Some(sc_subnet_id) => {
                let m = &ResourceMatcher::Id(vec![sc_subnet_id.clone()]);
                let subnets = match Subnet::describe(client, m).await {
                    Ok(subnets) => subnets,
                    Err(e) => panic!("[load_subnet] ERROR describe {:?}", e),
                };
                if subnets.is_empty() {
                    None
                } else {
                    Some(sc_subnet_id)
                }
            }
            None => None,
        };

        let final_subnet_id = match existing_subnet_id {
            None => {
                let Ok(subnet) = Subnet::create(client, sc).await else {
                    panic!("[load_subnet] Waaaah");
                };
                Some(subnet.subnet_id.unwrap().clone())
            }
            Some(subnet_id) => Some(subnet_id),
        };
        println!("[load_subnet] subnet_id {:?}", final_subnet_id.clone());
        Ok(final_subnet_id.unwrap().clone())
    }

    pub async fn load_security_group(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        println!("[load_security_group]");

        let existing_sg_id = match sc.security_group_id.clone() {
            Some(sc_security_group_id) => {
                let m = &ResourceMatcher::Id(vec![sc_security_group_id.clone()]);
                let security_groups = match SecurityGroup::describe(client, m).await {
                    Ok(sgs) => sgs,
                    Err(e) => panic!("[load_security_group] ERROR {:?}", e),
                };
                if security_groups.is_empty() {
                    None
                } else {
                    Some(sc_security_group_id)
                }
            }
            None => None,
        };

        let final_sg_id = match existing_sg_id {
            None => {
                let Ok(security_group) = SecurityGroup::create(client, sc).await else {
                    panic!("[load_security_group] Waaaah");
                };
                Some(security_group.clone())
            }
            Some(sg_id) => Some(sg_id),
        };
        println!(
            "[load_security_group] security_group_id {:?}",
            final_sg_id.as_ref().unwrap()
        );

        Ok(final_sg_id.unwrap().clone())
    }
}

#[derive(Debug, Clone)]
pub struct Swarm {
    config: SwarmConfig,
    network: AWSNetwork,
    key_pair: String,
    instances: Vec<Bee>,
}

impl Swarm {
    pub async fn load_swarm(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
    ) -> Result<Self, Error> {
        println!("[load_swarm]");

        let key_pair = match Swarm::load_key_pair(client, sc).await {
            Ok(key_id) => key_id,
            Err(e) => panic!("[load_swarm] ERROR load_key_pair\n{}", e),
        };
        let instances = match Swarm::load_instances(client, sc, network).await {
            Ok(instances) => instances.clone(),
            Err(e) => panic!("[load_swarm] ERROR load_instances\n{}", e),
        };

        Ok(Swarm {
            config: sc.clone(),
            network: network.clone(),
            key_pair: key_pair.clone(),
            instances: instances.clone(),
        })
    }

    pub async fn load_key_pair(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        println!("[load_key_pair]");

        let existing = match SSHKey::describe(client, aws::ec2::KEY_NAME).await {
            Ok(key_pairs) => key_pairs,
            Err(e) => panic!("[load_key_pair] ERROR describe {:?}", e),
        };

        if existing.is_empty() {
            let Ok(key_id) = SSHKey::import(client, sc, aws::ec2::KEY_NAME).await else {
                panic!("[load_key_pair] ERROR waaah");
            };
            Ok(key_id.clone())
        } else {
            // NOTE: would be better to handle multiple keys with some intention
            Ok(existing.first().unwrap().key_pair_id.clone().unwrap())
        }
    }

    pub async fn load_instances(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
    ) -> Result<Vec<Bee>, Error> {
        println!("[load_instances]");

        // load id and ip for all tagged instances
        let m = &BeeMatcher::Tagged(sc.tag_name.clone());
        let instances = match Instances::describe(client, m).await {
            Ok(instances) => instances.clone(),
            Err(e) => panic!("[load_swarm] ERROR load_instances\n{}", e),
        };
        println!("[load_instances] existing {}", instances.len());

        // create or terminate instances so count match appconfig
        let num_instances = instances.len() as i32;
        let beez = match sc.num_beez {
            // start additional instances
            num_beez if num_beez > num_instances => {
                let additional = num_beez - num_instances;
                println!("[load_instances] adding instances {}", additional);
                Instances::create(client, sc, network, Some(additional))
                    .await
                    .unwrap()
            }

            // terminate excess instances
            num_beez if num_beez < num_instances => {
                let excess = num_instances - num_beez;
                panic!("[load_instances] remove instances {}", excess);
            }

            // correct number are ready
            _ => {
                println!("[load_instances] right number instances");
                instances
            }
        };

        // wait for all to be fully initialized
        let beez = match Instances::wait_for_running(client, beez).await {
            Ok(instances) => instances.clone(),
            Err(e) => panic!("[load_instances] ERROR wait_for_running\n{}", e),
        };
        println!("[load_instances] swarm online");

        Ok(beez)
    }
}
