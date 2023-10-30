![the words "killa beez" are on top of a swarm of robotic bees](killabeez.jpg)

_A tool for using pools of EC2 instances to do all kinds of things._



## Overview

Create a pool of 20 EC2 instances, and call it `mytest`

```shell
$ beez pool init mytest --count 20
```

Use the `mytest` pool to execute a script and report back with output.

```shell
$ beez swarm mytest --script loadtest.sh
```

Terminate every instance in the `mytest` pool

```shell
$ beez pool terminate mytest
```


## Config

Environment variables:

* `AWS_ACCESS_KEY`
* `AWS_SECRET_KEY`


## Wu-Tang?

Yup. [Da Mystery Of Chessboxin'](https://youtu.be/pJk0p-98Xzc)

_EC2s, are you with me? Where you at?! In the front! In the back! Killa Beez on attack!_
