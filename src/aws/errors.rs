use std::fmt;

use aws_sdk_ec2::error::SdkError;
use aws_sdk_ec2::operation::{
    authorize_security_group_egress::AuthorizeSecurityGroupEgressError,
    authorize_security_group_ingress::AuthorizeSecurityGroupIngressError,
    create_internet_gateway::CreateInternetGatewayError,
    create_security_group::CreateSecurityGroupError, create_subnet::CreateSubnetError,
    create_vpc::CreateVpcError, delete_internet_gateway::DeleteInternetGatewayError,
    delete_key_pair::DeleteKeyPairError, delete_security_group::DeleteSecurityGroupError,
    delete_subnet::DeleteSubnetError, delete_vpc::DeleteVpcError,
    describe_instances::DescribeInstancesError,
    describe_internet_gateways::DescribeInternetGatewaysError,
    describe_key_pairs::DescribeKeyPairsError,
    describe_security_groups::DescribeSecurityGroupsError, describe_subnets::DescribeSubnetsError,
    describe_vpcs::DescribeVpcsError, import_key_pair::ImportKeyPairError,
    modify_subnet_attribute::ModifySubnetAttributeError, run_instances::RunInstancesError,
    terminate_instances::TerminateInstancesError,
};

#[derive(Debug)]
pub enum Ec2Error {
    // VPC
    CreateVpc(SdkError<CreateVpcError>),
    DeleteVpc(SdkError<DeleteVpcError>),
    DescribeVpcs(SdkError<DescribeVpcsError>),

    // Subnet
    CreateSubnet(SdkError<CreateSubnetError>),
    DeleteSubnet(SdkError<DeleteSubnetError>),
    DescribeSubnets(SdkError<DescribeSubnetsError>),
    ModifySubnetAttribute(SdkError<ModifySubnetAttributeError>),

    // Security Group
    CreateSecurityGroup(SdkError<CreateSecurityGroupError>),
    DeleteSecurityGroup(SdkError<DeleteSecurityGroupError>),
    DescribeSecurityGroups(SdkError<DescribeSecurityGroupsError>),

    // Internet Gateway
    CreateInternetGateway(SdkError<CreateInternetGatewayError>),
    DeleteInternetGateway(SdkError<DeleteInternetGatewayError>),
    DescribeInternetGateways(SdkError<DescribeInternetGatewaysError>),
    AuthorizeSecurityGroupIngress(SdkError<AuthorizeSecurityGroupIngressError>),
    AuthorizeSecurityGroupEgress(SdkError<AuthorizeSecurityGroupEgressError>),

    // SSH Key
    ImportSSHKey(SdkError<ImportKeyPairError>),
    DeleteSSHKey(SdkError<DeleteKeyPairError>),
    DescribeSSHKey(SdkError<DescribeKeyPairsError>),

    // Instances
    CreateInstances(SdkError<RunInstancesError>),
    TerminateInstances(SdkError<TerminateInstancesError>),
    DescribeInstances(SdkError<DescribeInstancesError>),

    Unexpected(String),
}

impl fmt::Display for Ec2Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // VPC
            Ec2Error::CreateVpc(err) => write!(f, "Failed to create vpc: {}", err),
            Ec2Error::DeleteVpc(err) => {
                write!(f, "Failed to delete vpc: {}", err)
            }
            Ec2Error::DescribeVpcs(err) => {
                write!(f, "Failed to describe vpcs: {}", err)
            }

            // Subnet
            Ec2Error::CreateSubnet(err) => write!(f, "Failed to create subnet: {}", err),
            Ec2Error::DeleteSubnet(err) => {
                write!(f, "Failed to delete subnet: {}", err)
            }
            Ec2Error::DescribeSubnets(err) => {
                write!(f, "Failed to describe subnets: {}", err)
            }
            Ec2Error::ModifySubnetAttribute(err) => write!(f, "Failed to modify subnet: {}", err),

            // Security Group
            Ec2Error::CreateSecurityGroup(err) => {
                write!(f, "Failed to create security group: {}", err)
            }
            Ec2Error::DeleteSecurityGroup(err) => {
                write!(f, "Failed to delete security group: {}", err)
            }
            Ec2Error::DescribeSecurityGroups(err) => {
                write!(f, "Failed to describe security groups: {}", err)
            }
            Ec2Error::AuthorizeSecurityGroupIngress(err) => {
                write!(f, "Failed to authorize ingress: {}", err)
            }
            Ec2Error::AuthorizeSecurityGroupEgress(err) => {
                write!(f, "Failed to authorize egress: {}", err)
            }

            // Internet Gateway
            Ec2Error::CreateInternetGateway(err) => {
                write!(f, "Failed to create security group: {}", err)
            }
            Ec2Error::DeleteInternetGateway(err) => {
                write!(f, "Failed to delete security group: {}", err)
            }
            Ec2Error::DescribeInternetGateways(err) => {
                write!(f, "Failed to describe security groups: {}", err)
            }

            // SSH Key
            Ec2Error::ImportSSHKey(err) => {
                write!(f, "Failed to import SSH key: {}", err)
            }
            Ec2Error::DeleteSSHKey(err) => {
                write!(f, "Failed to delete SSH key: {}", err)
            }
            Ec2Error::DescribeSSHKey(err) => {
                write!(f, "Failed to describe SSH key: {}", err)
            }

            // Instances
            Ec2Error::CreateInstances(err) => write!(f, "Failed to create instance: {}", err),
            Ec2Error::TerminateInstances(err) => {
                write!(f, "Failed to terminate instance: {}", err)
            }
            Ec2Error::DescribeInstances(err) => {
                write!(f, "Failed to describe instances: {}", err)
            }

            // No idea
            Ec2Error::Unexpected(msg) => write!(f, "Unexpected error: {}", msg),
        }
    }
}

impl std::error::Error for Ec2Error {}

// VPC

impl From<SdkError<CreateVpcError>> for Ec2Error {
    fn from(err: SdkError<CreateVpcError>) -> Self {
        Ec2Error::CreateVpc(err)
    }
}

impl From<SdkError<DeleteVpcError>> for Ec2Error {
    fn from(err: SdkError<DeleteVpcError>) -> Self {
        Ec2Error::DeleteVpc(err)
    }
}

impl From<SdkError<DescribeVpcsError>> for Ec2Error {
    fn from(err: SdkError<DescribeVpcsError>) -> Self {
        Ec2Error::DescribeVpcs(err)
    }
}

// Subnet

impl From<SdkError<CreateSubnetError>> for Ec2Error {
    fn from(err: SdkError<CreateSubnetError>) -> Self {
        Ec2Error::CreateSubnet(err)
    }
}

impl From<SdkError<DeleteSubnetError>> for Ec2Error {
    fn from(err: SdkError<DeleteSubnetError>) -> Self {
        Ec2Error::DeleteSubnet(err)
    }
}

impl From<SdkError<DescribeSubnetsError>> for Ec2Error {
    fn from(err: SdkError<DescribeSubnetsError>) -> Self {
        Ec2Error::DescribeSubnets(err)
    }
}

impl From<SdkError<ModifySubnetAttributeError>> for Ec2Error {
    fn from(err: SdkError<ModifySubnetAttributeError>) -> Self {
        Ec2Error::ModifySubnetAttribute(err)
    }
}

// Security Group

impl From<SdkError<CreateSecurityGroupError>> for Ec2Error {
    fn from(err: SdkError<CreateSecurityGroupError>) -> Self {
        Ec2Error::CreateSecurityGroup(err)
    }
}

impl From<SdkError<DeleteSecurityGroupError>> for Ec2Error {
    fn from(err: SdkError<DeleteSecurityGroupError>) -> Self {
        Ec2Error::DeleteSecurityGroup(err)
    }
}

impl From<SdkError<DescribeSecurityGroupsError>> for Ec2Error {
    fn from(err: SdkError<DescribeSecurityGroupsError>) -> Self {
        Ec2Error::DescribeSecurityGroups(err)
    }
}

// Internet Gateway

impl From<SdkError<CreateInternetGatewayError>> for Ec2Error {
    fn from(err: SdkError<CreateInternetGatewayError>) -> Self {
        Ec2Error::CreateInternetGateway(err)
    }
}

impl From<SdkError<DeleteInternetGatewayError>> for Ec2Error {
    fn from(err: SdkError<DeleteInternetGatewayError>) -> Self {
        Ec2Error::DeleteInternetGateway(err)
    }
}

impl From<SdkError<DescribeInternetGatewaysError>> for Ec2Error {
    fn from(err: SdkError<DescribeInternetGatewaysError>) -> Self {
        Ec2Error::DescribeInternetGateways(err)
    }
}

impl From<SdkError<AuthorizeSecurityGroupIngressError>> for Ec2Error {
    fn from(err: SdkError<AuthorizeSecurityGroupIngressError>) -> Self {
        Ec2Error::AuthorizeSecurityGroupIngress(err)
    }
}

impl From<SdkError<AuthorizeSecurityGroupEgressError>> for Ec2Error {
    fn from(err: SdkError<AuthorizeSecurityGroupEgressError>) -> Self {
        Ec2Error::AuthorizeSecurityGroupEgress(err)
    }
}

// SSH Key

impl From<SdkError<ImportKeyPairError>> for Ec2Error {
    fn from(err: SdkError<ImportKeyPairError>) -> Self {
        Ec2Error::ImportSSHKey(err)
    }
}

impl From<SdkError<DeleteKeyPairError>> for Ec2Error {
    fn from(err: SdkError<DeleteKeyPairError>) -> Self {
        Ec2Error::DeleteSSHKey(err)
    }
}

impl From<SdkError<DescribeKeyPairsError>> for Ec2Error {
    fn from(err: SdkError<DescribeKeyPairsError>) -> Self {
        Ec2Error::DescribeSSHKey(err)
    }
}

// Instances

impl From<SdkError<RunInstancesError>> for Ec2Error {
    fn from(err: SdkError<RunInstancesError>) -> Self {
        Ec2Error::CreateInstances(err)
    }
}

impl From<SdkError<DescribeInstancesError>> for Ec2Error {
    fn from(err: SdkError<DescribeInstancesError>) -> Self {
        Ec2Error::DescribeInstances(err)
    }
}

impl From<SdkError<TerminateInstancesError>> for Ec2Error {
    fn from(err: SdkError<TerminateInstancesError>) -> Self {
        Ec2Error::TerminateInstances(err)
    }
}
