use aws_sdk_ec2::waiters::security_group_exists;
use aws_sdk_ec2::{Client, Error};

use crate::aws;
use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct AWSNetwork {
    pub vpc_id: String,
    pub subnet_id: String,
    pub security_group_id: String,
}

impl AWSNetwork {
    pub async fn load_network(client: &Client, ac: &AppConfig) -> Result<Self, Error> {
        println!("[load_network]");

        let vpc_id = match AWSNetwork::load_vpc(client, ac).await {
            Ok(vpc_id) => vpc_id,
            Err(e) => panic!("[load_network] ERROR load_vpc\n{}", e),
        };
        let subnet_id = match AWSNetwork::load_subnet(client, ac).await {
            Ok(subnet_id) => subnet_id,
            Err(e) => panic!("[load_network] ERROR load_subnet\n{}", e),
        };
        let security_group_id = match AWSNetwork::load_security_group(client, ac).await {
            Ok(security_group_id) => security_group_id,
            Err(e) => panic!("[load_network] ERROR load_security_group\n{}", e),
        };

        Ok(AWSNetwork {
            vpc_id: vpc_id.clone(),
            subnet_id: subnet_id.clone(),
            security_group_id: security_group_id.clone(),
        })
    }

    pub async fn load_vpc(client: &Client, ac: &AppConfig) -> Result<String, Error> {
        println!("[load_vpc]");

        let existing_vpc_id = match ac.vpc_id.clone() {
            Some(ac_vpc_id) => {
                let Ok(vpcs) = aws::ec2::describe_vpc(client, &ac_vpc_id).await else {
                    panic!("[load_vpc] ERROR describe vpc_id failed {:?}", ac_vpc_id);
                };
                if vpcs.is_empty() {
                    None
                } else {
                    Some(ac_vpc_id)
                }
            }
            None => None,
        };

        println!("[load vpc] ac.vpc_id {:?}", existing_vpc_id.clone());

        let final_vpc_id = match existing_vpc_id {
            None => {
                let Ok(vpc) = aws::ec2::create_vpc(client, ac).await else {
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

    pub async fn load_subnet(client: &Client, ac: &AppConfig) -> Result<String, Error> {
        println!("[load_subnet]");

        let existing_subnet_id = match ac.subnet_id.clone() {
            Some(ac_subnet_id) => {
                let Ok(subnets) = aws::ec2::describe_subnet(client, &ac_subnet_id).await else {
                    panic!(
                        "[load_subnet] ERROR describe subnet_id failed {:?}",
                        ac_subnet_id
                    );
                };
                if subnets.is_empty() {
                    None
                } else {
                    Some(ac_subnet_id)
                }
            }
            None => None,
        };

        let final_subnet_id = match existing_subnet_id {
            None => {
                let Ok(subnet) = aws::ec2::create_subnet(client, ac).await else {
                    panic!("[load_subnet] Waaaah");
                };
                Some(subnet.subnet_id.unwrap().clone())
            }
            Some(subnet_id) => Some(subnet_id),
        };
        println!("[load_subnet] subnet_id {:?}", final_subnet_id.clone());
        Ok(final_subnet_id.unwrap().clone())
    }

    pub async fn load_security_group(client: &Client, ac: &AppConfig) -> Result<String, Error> {
        println!("[load_security_group]");

        let existing_sg_id = match ac.security_group_id.clone() {
            Some(ac_security_group_id) => {
                let Ok(security_groups) =
                    aws::ec2::describe_security_group(client, &ac_security_group_id).await
                else {
                    panic!(
                        "[load_security_group] ERROR describe security_group_id failed {:?}",
                        ac_security_group_id
                    );
                };
                if security_groups.is_empty() {
                    None
                } else {
                    Some(ac_security_group_id)
                }
            }
            None => None,
        };

        let final_sg_id = match existing_sg_id {
            None => {
                let Ok(security_group) = aws::ec2::create_security_group(client, ac).await else {
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
    network: AWSNetwork,
    key_pair: String,
    instances: Vec<String>,
}

impl Swarm {
    pub async fn init_swarm(
        client: &Client,
        ac: &AppConfig,
        network: &AWSNetwork,
    ) -> Result<Self, Error> {
        println!("[load_swarm]");

        let key_pair = match Swarm::load_key_pair(client, ac).await {
            Ok(key_id) => key_id,
            Err(e) => panic!("[load_swarm] ERROR load_key_pair\n{}", e),
        };
        let instance_ids = match Swarm::load_instances(client, ac, network).await {
            Ok(instance_ids) => instance_ids.clone(),
            Err(e) => panic!("[load_swarm] ERROR load_instances\n{}", e),
        };
        match aws::ec2::wait_for_instances(client, &instance_ids).await {
            Ok(_) => println!("[load_swarm] instances online"),
            Err(e) => panic!("[load_swarm] ERROR load_instances\n{}", e),
        };

        Ok(Swarm {
            network: network.clone(),
            key_pair: key_pair.clone(),
            instances: instance_ids.clone(),
        })
    }

    pub async fn load_key_pair(client: &Client, ac: &AppConfig) -> Result<String, Error> {
        println!("[load_key_pair]");

        let existing = match aws::ec2::describe_key_pair(client, aws::ec2::KEY_NAME).await {
            Ok(key_pairs) => key_pairs,
            Err(e) => panic!("[load_key_pair] ERROR describe {:?}", e),
        };

        if existing.is_empty() {
            let Ok(key_id) = aws::ec2::import_key_pair(client, ac, aws::ec2::KEY_NAME).await else {
                panic!("[load_key_pair] ERROR waaah");
            };
            Ok(key_id.clone())
        } else {
            Ok(existing.first().unwrap().key_pair_id.clone().unwrap())
        }
    }

    pub async fn load_instances(
        client: &Client,
        ac: &AppConfig,
        network: &AWSNetwork,
    ) -> Result<Vec<String>, Error> {
        println!("[load_instances]");

        let Ok(instance_ids) = aws::ec2::create_instances(
            client,
            &network.vpc_id,
            &network.subnet_id,
            &network.security_group_id,
            ac,
        )
        .await
        else {
            panic!("[load_instances] Waaaah!");
        };
        println!("[load_instances] instance ids created: {:?}", instance_ids);

        Ok(instance_ids)
    }
}
