![the words "killa beez" are on top of a swarm of robotic bees](docs/killabeez.jpg)

_A tool for using pools of EC2 instances to do all kinds of things._

This project is new and not yet complete. I am working through the foundational stuff and should have a full prototype together soon. As of now, all of the cloud management code works. Up next is controlling the instances via SSH.


## Overview 🏴‍☠️

Let's say you have a web system somewhere and you want to know how much load it can handle. You could do some math and talk about what _should_ happen. You could also just start slamming the thing and measure when it collapses. This project wants you to spit on your hands, hoist the black flag, and go hard for the latter.

The main steps:
1. Load or create a network on AWS
2. Load or create some number of EC2 instances (_a swarm_)
3. Use async pools of SSH connections to control swarm at scale
4. Use this foundation to create network traffic at arbitrary scale for load testing

Running `hostname` on 20 machines looks like this:

```shell
$ beez init
$ beez exec 'hostname'
$ beez terminate
```

A CLI param will exist soon that allows choosing a session config file, but for now it expects to see `sshpools.toml` in the dir where you run the command.


## Session Config

All commands will first read the session config file and then execute in the environment described by the config.

All of the resources created by killabeez are tagged. A config file can also be used to configure ids for vpc, subnet, security group, or SSH key. Tags are currently used to locate & manage EC2 instances according to the session config.

**Required Params**
- `username`: SSH account username for remote instances
- `tag_name`: string used to tag all remote resources
- `num_beez`: target count of instances running in pool

**Optional Params**
- `vpc_id`: use this VPC instead of creating one
- `subnet_id`: use this subnet instead of creating one
- `security_group_id`: use this security group instead of creating one
- `ami`: the OS image used to create instances
- `key_id`: use this key for SSH instead of importing one
- `key_file`: path to ssh public key to import (must be set if `key_id` is not)
- `ssh_cidr_block`: used to restrict SSH access to instances. it is recommended you limit access to just your machine which looks like: _your ip address/32_ or `11.22.33.44/32`


## Nah Mean

_EC2s, are you with me? Where you at?!_<br/>

![wu tang](docs/wutang.jpg)

_In the front! In the back! [Killa Beez](https://youtu.be/pJk0p-98Xzc) on attack!_

