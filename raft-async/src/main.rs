use std::time::Duration;

use async_std::sync::Arc;

use async_std::task;
use data::request::Request;
use server::{client::Client, server::Server, switch::Switch};
use state::state::StateMachine;

mod data;
mod server;
mod state;

#[derive(Default, Debug, Clone)]
struct Sum {
    total: u32,
}

impl StateMachine<u32, u32> for Sum {
    fn apply(&mut self, command: u32) {
        self.total += command;
    }

    fn get(&self) -> u32 {
        self.total
    }
}

fn main() {
    let switch: Arc<Switch<Request<u32, u32>>> = Switch::init();
    let servers = (0..5)
        .map(|id| Server::new(id, switch.clone()))
        .map(|server| Arc::new(task::block_on(server)))
        .collect::<Vec<_>>();
    let clients = (10..12)
        .map(|id| Client::<u32, u32>::new(id, switch.clone()))
        .map(|client| Arc::new(task::block_on(client)))
        .collect::<Vec<_>>();
    for server in servers {
        Server::<u32, u32>::init::<Sum>(server);
    }

    for client in clients {
        Client::init(client);
    }

    task::block_on(task::spawn(async {
        loop {
            task::sleep(Duration::from_secs(100)).await
        }
    }));

    // let servers = (0..5)
    //     .map(|i| RaftServer::<u32>::new(i))
    //     .collect::<Vec<_>>();
    // for i in 0..5 {
    //     for j in 0..5 {
    //         if i == j {
    //             continue;
    //         }
    //         let mut c1 = task::block_on(async { servers[i].output.lock().await });
    //         let mut c2 = task::block_on(async { servers[j].input.lock().await });
    //         c1.register_socket(&mut c2);
    //     }
    // }

    // let mut joins = Vec::new();
    // for mut server in servers {
    //     joins.push(task::spawn(async move {
    //         server.start().await;
    //     }));
    // }
    // for join in joins {
    //     task::block_on(join);
    // }
}
