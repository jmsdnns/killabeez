![the words "killa beez" are on top of a swarm of robotic bees](killabeez.jpg)

_A tool for using pools of EC2 instances to do all kinds of things._

This project is new. It needs to do some of the foundational stuff before it will feel polished, so bear with me as I go through those motions.

## Overview

Let's say you have a web system somewhere and you want to know how much load it can handle. You could do some math and talk about what _should_ happen. You could also try just start slamming the thing and measure what loads make it collapse. This project is built for the latter.

In terms of steps, we want to make it easy to turn on / off pools of servers used for load testing, eg. the _beez_. We then want it to be easy to run commands on each of them in parallel. The output is sent back to us where we do something with it and determine what to do next. It gives us the base functionality required for building particular types of tasks, such as HTTP load balancing.

```shell
$ beez init <name> --count 20
$ beez terminate <name>
$ beez exec <name> <cmd>
$ beez exec <name> --script <filepath>
```

Running `hostname` on 20 machines would look like this:

```shell
$ beez init thebeez --count 20
$ beez exec thebeez 'hostname'
$ beez terminate thebeez
```

Upload and then execute a script on every machine in the pool

```shell
$ beez exec thebeez --script loadtest.sh
```

## Nah Mean

_EC2s, are you with me? Where you at?!_

_In the front! In the back! [Killa Beez](https://youtu.be/pJk0p-98Xzc) on attack!_
