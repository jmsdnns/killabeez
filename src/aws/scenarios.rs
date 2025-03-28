use aws_sdk_ec2::waiters::security_group_exists;
use aws_sdk_ec2::{Client, Error, types::InstanceStateName};
use std::fmt;

use crate::aws::ec2::{
    Bee, BeeMatcher, Instances, InternetGateway, ResourceMatcher, SSHKey, SSHKeyMatcher,
    SecurityGroup, Subnet, Vpc,
};
use crate::aws::{self, ec2, errors::Ec2Error};
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
    pub async fn init(client: &Client, sc: &SwarmConfig) -> Result<Self, Ec2Error> {
        println!("[init_network]");

        let vpc_id = AWSNetwork::init_vpc(client, sc).await?;
        let subnet_id = AWSNetwork::init_subnet(client, sc, &vpc_id).await?;
        let security_group_id =
            AWSNetwork::init_security_group(client, sc, &vpc_id, &subnet_id).await?;
        let igw = AWSNetwork::init_internet_gateway(client, sc, &vpc_id).await?;

        Ok(AWSNetwork {
            vpc_id: vpc_id.to_owned(),
            subnet_id: subnet_id.to_owned(),
            security_group_id: security_group_id.to_owned(),
        })
    }

    async fn init_vpc(client: &Client, sc: &SwarmConfig) -> Result<String, Ec2Error> {
        match AWSNetwork::load_vpc(client, sc).await? {
            Some(vpc_id) => Ok(vpc_id),
            None => match Vpc::create(client, sc).await {
                Ok(vpc) => Ok(vpc.vpc_id.unwrap().to_owned()),
                Err(e) => Err(e),
            },
        }
    }

    async fn init_subnet(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<String, Ec2Error> {
        match AWSNetwork::load_subnet(client, sc, vpc_id).await? {
            Some(subnet_id) => Ok(subnet_id),
            None => match Subnet::create(client, sc, vpc_id).await {
                Ok(subnet) => Ok(subnet.subnet_id.unwrap().to_owned()),
                Err(e) => Err(e),
            },
        }
    }

    async fn init_security_group(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
        subnet_id: &str,
    ) -> Result<String, Ec2Error> {
        match AWSNetwork::load_security_group(client, sc, vpc_id, subnet_id).await? {
            Some(sg_id) => Ok(sg_id),
            None => match SecurityGroup::create(client, sc, vpc_id, subnet_id).await {
                Ok(sg_id) => Ok(sg_id.to_owned()),
                Err(e) => Err(e),
            },
        }
    }

    async fn init_internet_gateway(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<String, Ec2Error> {
        match AWSNetwork::load_internet_gateway(client, sc, vpc_id).await? {
            Some(igw_id) => Ok(igw_id),
            None => match InternetGateway::create(client, sc, vpc_id).await {
                Ok(igw) => Ok(igw.internet_gateway_id.unwrap().to_owned()),
                Err(e) => Err(e),
            },
        }
    }

    pub async fn load(client: &Client, sc: &SwarmConfig) -> Result<Self, Ec2Error> {
        println!("[load_network]");

        let vpc_id = AWSNetwork::load_vpc(client, sc).await?.unwrap();
        let subnet_id = AWSNetwork::load_subnet(client, sc, &vpc_id).await?.unwrap();
        let security_group_id = AWSNetwork::load_security_group(client, sc, &vpc_id, &subnet_id)
            .await?
            .unwrap();
        let igw = AWSNetwork::load_internet_gateway(client, sc, &vpc_id)
            .await?
            .unwrap();

        Ok(AWSNetwork {
            vpc_id: vpc_id.to_owned(),
            subnet_id: subnet_id.to_owned(),
            security_group_id: security_group_id.to_owned(),
        })
    }

    async fn load_vpc(client: &Client, sc: &SwarmConfig) -> Result<Option<String>, Ec2Error> {
        println!("[load_vpc]");

        match sc.vpc_id.clone() {
            None => {
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                let vpcs = Vpc::describe(client, m).await?;
                match vpcs.len() {
                    0 => Ok(None),
                    _ => Ok(vpcs.first().unwrap().vpc_id.clone()),
                }
            }
            Some(vpc_id) => {
                let m = ResourceMatcher::Id(vec![vpc_id]);
                let vpcs = Vpc::describe(client, m).await?;
                match vpcs.len() {
                    0 => Ok(None),
                    _ => Ok(vpcs.first().unwrap().vpc_id.clone()),
                }
            }
        }
    }

    async fn load_subnet(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<Option<String>, Ec2Error> {
        println!("[load_subnet]");

        match sc.subnet_id.clone() {
            None => {
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                let subnets = Subnet::describe(client, m).await?;
                match subnets.len() {
                    0 => Ok(None),
                    _ => Ok(subnets.first().unwrap().subnet_id.clone()),
                }
            }
            Some(subnet_id) => {
                let m = ResourceMatcher::Id(vec![subnet_id.clone()]);
                let subnets = Subnet::describe(client, m).await?;
                match subnets.len() {
                    0 => Ok(None),
                    _ => Ok(subnets.first().unwrap().subnet_id.clone()),
                }
            }
        }
    }

    async fn load_security_group(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
        subnet_id: &str,
    ) -> Result<Option<String>, Ec2Error> {
        println!("[load_security_group]");

        match sc.security_group_id.clone() {
            None => {
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                let sgs = SecurityGroup::describe(client, m).await?;
                match sgs.len() {
                    0 => Ok(None),
                    _ => Ok(sgs.first().unwrap().group_id.clone()),
                }
            }
            Some(sc_sg_id) => {
                let m = ResourceMatcher::Id(vec![sc_sg_id.clone()]);
                let sgs = SecurityGroup::describe(client, m).await?;
                match sgs.len() {
                    0 => Ok(None),
                    _ => Ok(sgs.first().unwrap().group_id.clone()),
                }
            }
        }
    }

    async fn load_internet_gateway(
        client: &Client,
        sc: &SwarmConfig,
        vpc_id: &str,
    ) -> Result<Option<String>, Ec2Error> {
        println!("[load_internet_gateway]");

        let m = ResourceMatcher::Tagged(sc.tag_name.clone());
        let igs = InternetGateway::describe(client, m).await?;
        match igs.len() {
            0 => Ok(None),
            _ => Ok(igs.first().unwrap().internet_gateway_id.clone()),
        }
    }

    pub async fn drop(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_network]");

        AWSNetwork::drop_internet_getway(client, sc).await?;
        AWSNetwork::drop_security_group(client, sc).await?;
        AWSNetwork::drop_subnet(client, sc).await?;
        AWSNetwork::drop_vpc(client, sc).await?;
        Ok(())
    }

    async fn drop_security_group(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_security_group]");

        match &sc.security_group_id {
            Some(sg_id) => Ok(()),
            None => {
                println!("[drop_security_group] fallback to tag");
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                SecurityGroup::delete(client, m.clone()).await
            }
        }
    }

    async fn drop_subnet(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_subnet]");
        match &sc.subnet_id {
            Some(subnet_id) => Ok(()),
            None => {
                println!("[drop_subnet] fallback to tag");
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                Subnet::delete(client, m.clone()).await
            }
        }
    }

    async fn drop_vpc(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_vpc]");
        match &sc.vpc_id {
            Some(vpc_id) => Ok(()),
            None => {
                println!("[drop_vpc] fallback to tag");
                let m = ResourceMatcher::Tagged(sc.tag_name.clone());
                Vpc::delete(client, m.clone()).await
            }
        }
    }

    async fn drop_internet_getway(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_internet_gateway]");
        let m = ResourceMatcher::Tagged(sc.tag_name.clone());
        InternetGateway::delete(client, m.clone()).await
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
    pub async fn init(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
    ) -> Result<Self, Ec2Error> {
        println!("[init_swarm]");

        let key_pair = Swarm::init_key_pair(client, sc).await?;
        let instances = Swarm::run_instances(client, sc, network).await?;

        Ok(Swarm {
            config: sc.to_owned(),
            network: network.to_owned(),
            key_pair: key_pair.to_owned(),
            instances: instances.to_owned(),
        })
    }

    async fn init_key_pair(client: &Client, sc: &SwarmConfig) -> Result<String, Ec2Error> {
        match Swarm::load_key_pair(client, sc).await? {
            Some(key_id) => Ok(key_id),
            None => SSHKey::import(client, sc).await,
        }
    }

    pub async fn load(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
    ) -> Result<Self, Ec2Error> {
        println!("[load_swarm]");

        let key_pair = Swarm::load_key_pair(client, sc).await?.unwrap();
        let instances = Swarm::run_instances(client, sc, network).await?;

        Ok(Swarm {
            config: sc.to_owned(),
            network: network.to_owned(),
            key_pair: key_pair.to_owned(),
            instances: instances.to_owned(),
        })
    }

    async fn load_key_pair(client: &Client, sc: &SwarmConfig) -> Result<Option<String>, Ec2Error> {
        println!("[load_key_pair]");

        match sc.key_id.clone() {
            None => {
                let m = SSHKeyMatcher::Name(sc.tag_name.clone());
                let keys = SSHKey::describe(client, m).await?;
                match keys.len() {
                    0 => Ok(None),
                    _ => Ok(keys.first().unwrap().key_pair_id.clone()),
                }
            }
            Some(key_id) => {
                let m = SSHKeyMatcher::Id(key_id.clone());
                let keys = SSHKey::describe(client, m).await?;
                match keys.len() {
                    0 => Ok(None),
                    _ => Ok(keys.first().unwrap().key_pair_id.clone()),
                }
            }
        }
    }

    async fn run_instances(
        client: &Client,
        sc: &SwarmConfig,
        network: &AWSNetwork,
    ) -> Result<Vec<Bee>, Ec2Error> {
        println!("[run_instances]");

        let target_state = InstanceStateName::Running;

        // load id and ip for all tagged instances
        let m = BeeMatcher::Tagged(sc.tag_name.clone());
        let instances = Instances::describe(client, m, target_state.clone()).await?;
        println!("[run_instances] existing {}", instances.len());

        // create or terminate instances so count match appconfig
        let num_instances = instances.len() as i32;
        let loaded_beez: Result<Vec<Bee>, Ec2Error> = match sc.num_beez {
            // start additional instances
            num_beez if num_beez > num_instances => {
                let additional = num_beez - num_instances;
                println!("[run_instances] adding instances {}", additional);
                let new_beez = Instances::create(client, sc, network, Some(additional)).await?;
                Ok([instances, new_beez].concat())
            }

            // terminate excess instances
            num_beez if num_beez < num_instances => {
                let excess = num_instances - num_beez;
                let rip_instances = instances
                    .clone()
                    .drain(0..(excess as usize))
                    .collect::<Vec<Bee>>();
                Instances::delete(client, sc, &BeeMatcher::Ids(rip_instances.clone())).await?;
                Ok(instances.to_owned())
            }

            // correct number already running
            _ => {
                println!("[run_instances] right number instances");
                Ok(instances.to_owned())
            }
        };

        // wait for all to be fully initialized
        let ids = Instances::wait(client, loaded_beez.unwrap(), target_state).await?;
        Ok(ids.to_owned())
    }

    pub async fn drop(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_swarm]");

        Swarm::drop_instances(client, sc).await?;
        Swarm::drop_key_pair(client, sc).await?;
        Ok(())
    }

    async fn drop_instances(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_instances]");
        let m = BeeMatcher::Tagged(sc.tag_name.clone());
        let beez = Instances::delete(client, sc, &m.clone()).await?;

        // wait for all to be fully terminated
        Instances::wait(client, beez.clone(), InstanceStateName::Terminated).await?;
        Ok(())
    }

    async fn drop_key_pair(client: &Client, sc: &SwarmConfig) -> Result<(), Ec2Error> {
        println!("[drop_key_pair]");
        match &sc.key_id.clone() {
            Some(key_id) => Ok(()),
            None => {
                println!("[drop_key_pair] fallback to tag");
                let m = SSHKeyMatcher::Name(sc.tag_name.clone());
                SSHKey::delete(client, m.clone()).await
            }
        }
    }
}
