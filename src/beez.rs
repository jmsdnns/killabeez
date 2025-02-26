use aws_sdk_ec2::{Client, Error};
use tokio;

use crate::aws;
use crate::config::AppConfig;

pub async fn mk_client(ac: &AppConfig) -> Result<Client, Error> {
    let config = aws_config::load_from_env().await;
    Ok(Client::new(&config))
}

pub struct AWSNetwork {
    pub vpc_id: String,
    pub subnet_id: String,
    pub security_group_id: String,
}

impl AWSNetwork {
    async fn mk_network(client: &Client, ac: &AppConfig) -> Result<Self, Error> {
        // VPC
        let Ok(vpc) = aws::create_vpc(&client, &ac).await else {
            panic!("[vpc] Waaaah");
        };
        let vpc_id = vpc.vpc_id.as_ref().unwrap();
        println!("VPC created: {:?}", vpc_id);

        // Subnet
        let Ok(subnet) = aws::create_subnet(&client, vpc_id, &ac).await else {
            panic!("[subnet] Waaaah!");
        };
        let subnet_id = subnet.subnet_id.as_ref().unwrap();
        println!("Subnet created: {:?}", subnet_id);

        // Security Group
        let Ok(sg_id) = aws::create_security_group(&client, vpc_id, &ac).await else {
            panic!("[security_group] Waaaah!");
        };
        println!("Security Group created: {:?}", sg_id);

        Ok(AWSNetwork {
            vpc_id: vpc_id.clone(),
            subnet_id: subnet_id.clone(),
            security_group_id: sg_id.clone(),
        })
    }
}

pub struct Swarm {}

impl Swarm {
    async fn mk_swarm(client: &Client, ac: AppConfig, network: AWSNetwork) -> Result<(), Error> {
        // Key Pair
        let Ok(key_pair_id) = aws::import_key_pair(&client, &ac).await else {
            panic!("[key pair] Waaaah!");
        };
        println!("Key Pair created: {:?}", key_pair_id);

        // EC2 Instances
        let Ok(instance_ips) = aws::create_instances(
            &client,
            &network.vpc_id,
            &network.subnet_id,
            &network.security_group_id,
            &ac,
        )
        .await
        else {
            panic!("[instances] Waaaah!");
        };
        println!("Instances created, IPs: {:?}", instance_ips);

        Ok(())
    }
}
