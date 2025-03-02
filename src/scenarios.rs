use aws_sdk_ec2::waiters::security_group_exists;
use aws_sdk_ec2::{Client, Error};

use crate::aws;
use crate::aws::ec2::Bee;
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
                let Ok(vpcs) = aws::ec2::describe_vpc(client, &sc_vpc_id).await else {
                    panic!("[load_vpc] ERROR describe vpc_id failed {:?}", sc_vpc_id);
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
                let Ok(vpc) = aws::ec2::create_vpc(client, sc).await else {
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
                let Ok(subnets) = aws::ec2::describe_subnet(client, &sc_subnet_id).await else {
                    panic!(
                        "[load_subnet] ERROR describe subnet_id failed {:?}",
                        sc_subnet_id
                    );
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
                let Ok(subnet) = aws::ec2::create_subnet(client, sc).await else {
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
                let Ok(security_groups) =
                    aws::ec2::describe_security_group(client, &sc_security_group_id).await
                else {
                    panic!(
                        "[load_security_group] ERROR describe security_group_id failed {:?}",
                        sc_security_group_id
                    );
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
                let Ok(security_group) = aws::ec2::create_security_group(client, sc).await else {
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
    pub async fn init_swarm(
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
        let instances = match aws::ec2::wait_for_running(client, instances).await {
            Ok(instances) => instances.clone(),
            Err(e) => panic!("[load_swarm] ERROR wait_for_running\n{}", e),
        };
        println!("[load_swarm] swarm online");

        Ok(Swarm {
            config: sc.clone(),
            network: network.clone(),
            key_pair: key_pair.clone(),
            instances: instances.clone(),
        })
    }

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
        let instances = match aws::ec2::load_tagged(client, sc).await {
            Ok(instances) => instances.clone(),
            Err(e) => panic!("[load_swarm] ERROR load_instances\n{}", e),
        };
        println!("[load_swarm] swarm online");

        Ok(Swarm {
            config: sc.clone(),
            network: network.clone(),
            key_pair: key_pair.clone(),
            instances: instances.clone(),
        })
    }

    pub async fn load_key_pair(client: &Client, sc: &SwarmConfig) -> Result<String, Error> {
        println!("[load_key_pair]");

        let existing = match aws::ec2::describe_key_pair(client, aws::ec2::KEY_NAME).await {
            Ok(key_pairs) => key_pairs,
            Err(e) => panic!("[load_key_pair] ERROR describe {:?}", e),
        };

        if existing.is_empty() {
            let Ok(key_id) = aws::ec2::import_key_pair(client, sc, aws::ec2::KEY_NAME).await else {
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

        // 1. load id and ip for all tagged instances
        // 2. create or terminate instances so count match appconfig
        // 3. return id & ip

        // let Ok(instance_ids) = aws::ec2::create_instances(
        //     client,
        //     &network.vpc_id,
        //     &network.subnet_id,
        //     &network.security_group_id,
        //     sc,
        // )
        // .await
        // else {
        //     panic!("[load_instances] Waaaah!");
        // };
        // println!("[load_instances] instance ids created: {:?}", instance_ids);

        // Ok(instance_ids)
        todo!()
    }
}
