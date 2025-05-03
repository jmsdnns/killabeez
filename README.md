_A tool for controlling pools of EC2 instances via SSH and SFTP._

![the words "killa beez" are on top of a swarm of robotic bees](docs/killabeez.jpg)

Let's say you have a web system somewhere and you want to know how much load it can handle. You could do some math and talk about what _should_ happen. You could also just start slamming the thing and measure when it collapses. This project wants you to spit on your hands, hoist the black flag, and slam the crap out of something.


## Overview 🏴‍☠️

The main considerations:

1. We need AWS resources
2. We want to control some number of EC2 instances in parallel (via SSH & SFTP)
3. We want to use this foundation to create execution plans for coordinated behaviors
4. Flexible configuration
5. Option to stream output to local files or stash output remotely and collect it all after

Going from nothing to running `ls -a` on 20 new instances looks like this:

```
$ beez init
$ beez exec "ls -a"
$ beez terminate
```


## CLI

```
killabeez: a CLI for creating traffic jams of arbitrary scale

Usage: beez <COMMAND>

Commands:
  init       Prepares resources for bringing swarm online
  tagged     Lists all resources tagged by swarm config
  terminate  Terminate all managed resources
  exec       Execute command on swarm
  upload     Upload a file to swarm
  download   Download a file from swarm
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```


## Swarm Config

A pool of instances is called a _swarm_ and they are configured with a _swarm config_.

**Required Params**

- `tag_name`: string used to tag all remote resources
- `num_beez`: target count of ec2 instances in pool

Beez looks for a `swarm.toml` in the current directory by default but this can be overridden with `-c`. Running multiple swarms is as easy as having multiple configs, each with a unique tag name.

```
beez exec -c myswarm.toml "ls -a"
```

There are more optional params available, mostly concerned with resource management, so they are explained in the next section.


## Execution Plans

I have built a parser for plans that hasn't been integrated yet. The _plan_ is to read a config like this and execute everything in a single SSH session.

```yaml
execute: echo "Take Five!"
upload:
  local_path: myscript.sh
  remote_path: /tmp/myscript.sh
download:
  remote_path: /tmp/all-the-output.tar.gz
  local_path: takefive.tar.gz
```

Here is the prototype [plan parser](https://gist.github.com/jmsdnns/a83255fa1b0f0232e91c96f205a6a4ea). I just need more time to integrate it with beez.


## Resource Management

All resources required for running swarms can be created and managed by beez, but it is also possible to use an existing resource, like a team VPC, by adding its ID to the swarm config. All resources managed by beez are tagged with the `tag_name` so they can be discovered & reused in multiple sessions and easily terminated whenever you're done.

For a user, this means:

- beez can create everything from scratch and clean up after itself
- beez can also work in more complicated environments with stricter access, requiring only the ability to turn ec2 instances on / off
- resources wont be created and forgotten about

Here's the list of optional params for swarm configs that we mentioned earlier:

**Optional Params**
- `vpc_id`: use this VPC instead of creating one
- `subnet_id`: use this subnet instead of creating one
- `security_group_id`: use this security group instead of creating one
- `ami`: the OS image used to create instances
- `username`: SSH account username for remote instances
- `key_id`: use this key for SSH instead of importing one
- `key_file`: path to ssh public key to import (must be set if `key_id` is not)
- `ssh_cidr_block`: used to restrict SSH access to instances. it is recommended you limit access to just your machine which looks like: _your ip address/32_ or `11.22.33.44/32`



## Nah Mean

_EC2s, are you with me? Where you at?!_<br/>

![WU TANG](docs/wutang.jpg)

_In the front! In the back! [Killa Beez](https://youtu.be/pJk0p-98Xzc) on attack!_

