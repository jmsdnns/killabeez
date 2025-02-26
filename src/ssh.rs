use async_ssh2_tokio::{
    client::{AuthMethod, Client, CommandExecutedResult, ServerCheckMethod},
    Error,
};
use futures::{stream, StreamExt};

pub struct SSHConnection {
    client: Client,
    host: String,
    username: String,
}

impl SSHConnection {
    pub async fn open(host: &str, username: &str, auth: &AuthMethod) -> Self {
        let dst = match host.split(":").collect::<Vec<&str>>()[..] {
            [h, p] => (h, p.parse::<u16>().unwrap()),
            [h] => (h, 22),
            _ => panic!("Host value makes no sense: {}", host),
        };

        let conn = Client::connect(dst, username, auth.clone(), ServerCheckMethod::NoCheck).await;

        //println!("CONN: {:?}", conn);

        SSHConnection {
            client: conn.unwrap(),
            host: String::from(host),
            username: String::from(username),
        }
    }
}

pub struct SSHPool {
    conns: Vec<SSHConnection>,
}

impl SSHPool {
    pub async fn new(hosts: &Vec<&str>, username: &str, auth: &AuthMethod) -> SSHPool {
        let concurrency: usize = 10;

        let results = stream::iter(hosts)
            .map(|host| SSHConnection::open(hosts, username, &auth))
            .buffer_unordered(concurrency)
            .collect::<Vec<SSHConnection>>()
            .await;

        SSHPool { conns: results }
    }

    pub async fn exec(&self, cmd: &str) -> Vec<CommandExecutedResult> {
        let results = stream::iter(self.conns.iter())
            .map(|c| c.client.execute(cmd))
            .buffer_unordered(10)
            .collect::<Vec<Result<CommandExecutedResult, Error>>>()
            .await;

        let mut output = Vec::new();
        for r in results.iter() {
            output.push(r.as_ref().unwrap().clone());
        }

        output
    }
}

pub fn print_results(results: Vec<CommandExecutedResult>) {
    for r in results.iter() {
        print!("{}", r.stdout);
    }
}
