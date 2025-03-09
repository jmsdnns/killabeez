use aws_sdk_ec2::waiters::security_group_exists;
use aws_sdk_ec2::{Client, Error, types::InstanceStateName};
use std::fmt;

use crate::aws::ec2::{
    Bee, BeeMatcher, Instances, ResourceMatcher, SSHKey, SSHKeyMatcher, SecurityGroup, Subnet, VPC,
};
use crate::aws::{self, ec2};
use crate::config::SwarmConfig;

#[derive(Debug, Clone)]
pub struct AWSNetwork {
    pub vpc_id: String,
    pub subnet_id: String,
    pub security_group_id: String,
}

impl fmt::Display for AWSNetwork {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "NETWORK ]--------------------------\n\
             VPC ID:    {}\n\
             Subnet ID: {}\n\
             SG ID:     {}",
            self.vpc_id, self.subnet_id, self.security_group_id,
        )
    }
}

impl AWSNetwork {
    pub async fn load_network(client: &Client, sc: &SwarmConfig) -> Result<Self, Error> {
        println!("[load_network]");

        let vpc_id = match AWSNetwork::load_vpc(client, sc).await {
            Ok(vpc_id) => vpc_id,
            Err(e) => unimplemented!(),
        };
        let subnet_id = match AWSNetwork::load_subnet(client, sc, &vpc_id).await {
            Ok(subnet_id) => subnet_id,
            Err(e) => unimplemented!(),
        };
        let security_group_id =
            match AWSNetwork::load_security_group(client, sc, &vpc_id, &subnet_id).await {
                Ok(security_group_id) => security_group_id,
                Err(e) => unimplemented!(),
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
                let m = ResourceMatcher::Id(vec![sc_vpc_id.clone()]);
                match VPC::describe(client, m).await {
                    Ok(vpcs) => match vpcs.len() {
                        0 => None,
                        _ => Some(sc_vpc_id),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            None => None,
        };

        let existing_vpc_id = match existing_vpc_id {
            None => {
                // try loading tag name
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                match VPC::describe(client, m).await {
                    Ok(vpcs) => match vpcs.len() {
                        0 => None,
                        _ => vpcs.first().unwrap().vpc_id.clone(),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            Some(vpc_id) => Some(vpc_id.clone()),
        };

        println!("[load vpc] sc.vpc_id {:?}", existing_vpc_id.clone());

        match existing_vpc_id {
            None => match VPC::create(client, sc).await {
                Ok(vpc) => Ok(vpc.vpc_id.unwrap().clone()),
                Err(e) => unimplemented!(),
            },
            Some(vpc_id) => Ok(vpc_id),
        }
    }

    pub async fn load_subnet(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<String, Error> {
        println!("[load_subnet]");

        let existing_subnet_id = match sc.subnet_id.clone() {
            Some(sc_subnet_id) => {
                let m = ResourceMatcher::Id(vec![sc_subnet_id.clone()]);
                match Subnet::describe(client, m).await {
                    Ok(subnets) => match subnets.len() {
                        0 => None,
                        _ => Some(sc_subnet_id),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            None => None,
        };

        let existing_subnet_id = match existing_subnet_id {
            None => {
                // try loading tag name
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                match Subnet::describe(client, m).await {
                    Ok(subnets) => match subnets.len() {
                        0 => None,
                        _ => subnets.first().unwrap().subnet_id.clone(),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            Some(subnet_id) => Some(subnet_id.clone()),
        };

        match existing_subnet_id {
            None => match Subnet::create(client, sc, vpc_id).await {
                Ok(subnet) => Ok(subnet.subnet_id.unwrap().clone()),
                Err(e) => unimplemented!(),
            },
            Some(subnet_id) => Ok(subnet_id),
        }
    }

    pub async fn load_security_group(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
        subnet_id: &str,
    ) -> Result<String, Error> {
        println!("[load_security_group]");

        let existing_sg_id = match sc.security_group_id.clone() {
            Some(sc_security_group_id) => {
                let m = ResourceMatcher::Id(vec![sc_security_group_id.clone()]);
                match SecurityGroup::describe(client, m).await {
                    Ok(sgs) => match sgs.len() {
                        0 => None,
                        _ => Some(sc_security_group_id),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            None => None,
        };

        let existing_sg_id = match existing_sg_id {
            None => {
                // try loading tag name
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                match SecurityGroup::describe(client, m).await {
                    Ok(security_groups) => match security_groups.len() {
                        0 => None,
                        _ => security_groups.first().unwrap().group_id.clone(),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            Some(sg_id) => Some(sg_id.clone()),
        };

        match existing_sg_id {
            None => match SecurityGroup::create(client, sc, vpc_id, subnet_id).await {
                Ok(sg_id) => Ok(sg_id.clone()),
                Err(e) => unimplemented!(),
            },
            Some(sg_id) => Ok(sg_id.clone()),
        }
    }

    pub async fn drop_network(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_network]");

        let typed_ok: Result<(), Error> = Ok(());

        match AWSNetwork::drop_security_group(client, sc).await {
            Ok(()) => &typed_ok,
            Err(e) => unimplemented!(),
        };
        match AWSNetwork::drop_subnet(client, sc).await {
            Ok(()) => &typed_ok,
            Err(e) => unimplemented!(),
        };
        match AWSNetwork::drop_vpc(client, sc).await {
            Ok(()) => &typed_ok,
            Err(e) => unimplemented!(),
        };

        typed_ok
    }

    pub async fn drop_security_group(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_security_group]");
        // let typed_ok: Result<(), Error> = Ok(());

        match &sc.security_group_id {
            // ID found in config
            Some(sg_id) => Ok(()),
            // No ID in config, try deleting by tag
            None => {
                println!("[drop_security_group] fallback to tag");
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                match SecurityGroup::delete(client, m.clone()).await {
                    Ok(()) => Ok(()),
                    Err(e) => unimplemented!(),
                }
            }
        }
    }

    pub async fn drop_subnet(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_subnet]");
        match &sc.subnet_id {
            Some(subnet_id) => Ok(()),
            None => {
                println!("[drop_subnet] fallback to tag");
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                match Subnet::delete(client, m.clone()).await {
                    Ok(()) => Ok(()),
                    Err(e) => unimplemented!(),
                }
            }
        }
    }

    pub async fn drop_vpc(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_vpc]");
        match &sc.vpc_id {
            Some(vpc_id) => Ok(()),
            None => {
                println!("[drop_vpc] fallback to tag");
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                match VPC::delete(client, m.clone()).await {
                    Ok(()) => Ok(()),
                    Err(e) => unimplemented!(),
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Swarm {
    pub config: SwarmConfig,
    pub network: AWSNetwork,
    pub key_pair: String,
    pub instances: Vec<Bee>,
}

impl fmt::Display for Swarm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SWARM ]----------------------------\n\
             Instances: {}\n\
             SSH Key:   {}\n\
             {}",
            self.instances
                .iter()
                .map(|b| b.ip.clone().unwrap())
                .collect::<Vec<String>>()
                .join(", "),
            self.key_pair,
            self.network,
        )
    }
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
            Err(e) => unimplemented!(),
        };
        let instances = match Swarm::load_instances(client, sc, network).await {
            Ok(instances) => instances.clone(),
            Err(e) => unimplemented!(),
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

        // key id found in config, try to load key
        let existing_key_id = match sc.key_id.clone() {
            Some(key_id) => {
                let m = SSHKeyMatcher::Id(key_id.clone());
                match SSHKey::describe(client, m).await {
                    Ok(key_pairs) => match key_pairs.len() {
                        0 => None,
                        _ => Some(key_pairs.first().unwrap().key_pair_id.clone().unwrap()),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            None => None,
        };

        // no key id found in config, try loading by name (tag_name)
        let existing_key_id = match existing_key_id {
            None => {
                // try loading tag name
                let m = SSHKeyMatcher::Name(sc.tag_name.clone());
                match SSHKey::describe(client, m).await {
                    Ok(key_infos) => match key_infos.len() {
                        0 => None,
                        _ => key_infos.first().unwrap().key_pair_id.clone(),
                    },
                    Err(e) => unimplemented!(),
                }
            }
            Some(key_id) => Some(key_id.clone()),
        };

        // no key found so far, check if config has `key_file` and import it
        match existing_key_id {
            None => match sc.key_file.clone() {
                Some(key) => match SSHKey::import(client, sc).await {
                    Ok(key_id) => Ok(key_id),
                    Err(e) => unimplemented!(),
                },
                None => unimplemented!(),
            },
            Some(key_id) => Ok(key_id),
        }
    }

    pub async fn load_instances(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
    ) -> Result<Vec<Bee>, Error> {
        println!("[load_instances]");

        // load id and ip for all tagged instances
        let m = BeeMatcher::Tagged(sc.tag_name.clone());
        let mut instances = match Instances::describe(client, m, InstanceStateName::Running).await {
            Ok(instances) => instances.clone(),
            Err(e) => unimplemented!(),
        };
        println!("[load_instances] existing {}", instances.len());

        // create or terminate instances so count match appconfig
        let num_instances = instances.len() as i32;
        let loaded_beez = match sc.num_beez {
            // start additional instances
            num_beez if num_beez > num_instances => {
                let additional = num_beez - num_instances;
                println!("[load_instances] adding instances {}", additional);
                match Instances::create(client, sc, network, Some(additional)).await {
                    Ok(new_beez) => [instances, new_beez].concat(),
                    Err(e) => unimplemented!(),
                }
            }

            // terminate excess instances
            num_beez if num_beez < num_instances => {
                let excess = num_instances - num_beez;
                let rip_instances = instances.drain(0..(excess as usize)).collect::<Vec<Bee>>();
                match Instances::delete(client, sc, &BeeMatcher::Ids(rip_instances)).await {
                    Ok(_) => instances,
                    Err(e) => unimplemented!(),
                }
            }

            // correct number are ready
            _ => {
                println!("[load_instances] right number instances");
                instances
            }
        };

        // wait for all to be fully initialized
        match Instances::wait_for_state(client, loaded_beez, InstanceStateName::Running).await {
            Ok(instances) => Ok(instances.clone()),
            Err(e) => unimplemented!(),
        }
    }

    pub async fn drop_swarm(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_swarm]");

        let typed_ok: Result<(), Error> = Ok(());

        match Swarm::drop_instances(client, sc).await {
            Ok(()) => &typed_ok,
            Err(e) => unimplemented!(),
        };
        match Swarm::drop_key_pair(client, sc).await {
            Ok(()) => &typed_ok,
            Err(e) => unimplemented!(),
        };

        typed_ok
    }

    pub async fn drop_instances(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_instances]");
        let m = BeeMatcher::Tagged(sc.tag_name.clone());
        let beez = match Instances::delete(client, sc, &m.clone()).await {
            Ok(beez) => beez,
            Err(e) => unimplemented!(),
        };
        // wait for all to be fully initialized
        match Instances::wait_for_state(client, beez.clone(), InstanceStateName::Terminated).await {
            Ok(_) => Ok(()),
            Err(e) => unimplemented!(),
        }
    }

    pub async fn drop_key_pair(client: &Client, sc: &SwarmConfig) -> Result<(), Error> {
        println!("[drop_key_pair]");
        match &sc.key_id.clone() {
            Some(key_id) => Ok(()),
            None => {
                println!("[drop_key_pair] fallback to tag");
                let m = SSHKeyMatcher::Name(sc.tag_name.clone());
                match SSHKey::delete(client, m.clone()).await {
                    Ok(()) => Ok(()),
                    Err(e) => unimplemented!(),
                }
            }
        }
    }
}
