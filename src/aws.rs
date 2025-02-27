use aws_config::{ConfigLoader, meta::region::RegionProviderChain};
use aws_sdk_ec2::{
    Client, Error,
    client::Waiters,
    error::SdkError,
    operation::create_vpc::CreateVpcError,
    types::{
        Instance, InstanceType, KeyPairInfo, ResourceType, SecurityGroup, Subnet, Tag,
        TagSpecification, Vpc, builders::TagSpecificationBuilder,
    },
};
use std::collections::HashMap;

use crate::config::AppConfig;

fn create_tag_spec(ac: &AppConfig) -> TagSpecification {
    TagSpecification::builder()
        .resource_type(ResourceType::Vpc)
        .tags(
            Tag::builder()
                .key("Name")
                .value(ac.tag_name.clone())
                .build(),
        )
        .build()
}

pub async fn create_vpc(client: &Client, ac: &AppConfig) -> Result<Vpc, Error> {
    let tag_specifications = create_tag_spec(ac);

    let response = client
        .create_vpc()
        .cidr_block("10.0.0.0/16")
        .tag_specifications(tag_specifications)
        .send()
        .await?;

    Ok(response.vpc.unwrap())
}

pub async fn create_subnet(client: &Client, vpc_id: &str, ac: &AppConfig) -> Result<Subnet, Error> {
    let tag_specifications = create_tag_spec(ac);
    let cidr_block = ac.cidr_block.as_ref().unwrap();

    let response = client
        .create_subnet()
        .cidr_block(cidr_block)
        .vpc_id(vpc_id)
        .availability_zone("us-east-1a")
        .tag_specifications(tag_specifications)
        .send()
        .await?;

    Ok(response.subnet.unwrap())
}

pub async fn create_security_group(
    client: &Client,
    vpc_id: &str,
    ac: &AppConfig,
) -> Result<String, Error> {
    let tag_specifications = create_tag_spec(ac);
    let cidr_block = ac.cidr_block.as_ref().unwrap();

    let response = client
        .create_security_group()
        .vpc_id(vpc_id)
        .group_name("allow_ssh")
        .description("Allow SSH inbound traffic")
        .tag_specifications(tag_specifications)
        .send()
        .await?;

    let sg_id = response.group_id.unwrap();

    // Add ingress rule to allow SSH
    client
        .authorize_security_group_ingress()
        .group_id(&sg_id)
        .set_ip_protocol(Some("tcp".to_string()))
        .from_port(22)
        .to_port(22)
        .cidr_ip(cidr_block)
        .send()
        .await?;

    // Add egress rule to allow all outbound traffic
    client
        .authorize_security_group_egress()
        .group_id(&sg_id)
        .set_ip_protocol(Some("-1".to_string()))
        .from_port(0)
        .to_port(0)
        .cidr_ip("0.0.0.0/0")
        .send()
        .await?;

    Ok(sg_id)
}

pub async fn import_key_pair(client: &Client, ac: &AppConfig) -> Result<String, Error> {
    let tag_specifications = create_tag_spec(ac);

    let Some(key_material) = std::fs::read_to_string(ac.key_file.clone()).ok() else {
        panic!("[key material] Waaaah!");
    };

    let key_blob = aws_sdk_ec2::primitives::Blob::new(key_material);

    let Ok(response) = client
        .import_key_pair()
        .key_name("the beez kees")
        .public_key_material(key_blob)
        .tag_specifications(tag_specifications)
        .send()
        .await
    else {
        panic!("[key pair] Waaaah!");
    };

    Ok(response.key_pair_id.unwrap())
}

pub async fn create_instances(
    client: &Client,
    vpc_id: &str,
    subnet_id: &str,
    sg_id: &str,
    ac: &AppConfig,
) -> Result<Vec<String>, Error> {
    let tag_specifications = create_tag_spec(ac);

    let mut instance_ips = Vec::new();

    for i in 0..ac.num_beez {
        let create_instance_response = client
            .run_instances()
            .instance_type(InstanceType::T2Micro)
            .image_id("ami-0c55b159cbfafe1f0")
            .key_name("the beez kees")
            .subnet_id(subnet_id)
            .security_group_ids(sg_id)
            .tag_specifications(tag_specifications.clone())
            .min_count(ac.num_beez)
            .max_count(ac.num_beez)
            .send()
            .await?;

        let instance = create_instance_response.instances.unwrap()[0].clone();
        instance_ips.push(instance.public_ip_address.unwrap());
    }

    Ok(instance_ips)
}
