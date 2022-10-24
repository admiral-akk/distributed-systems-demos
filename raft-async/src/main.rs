use std::time::Duration;

use async_std::sync::Arc;

use async_std::task;
use server::{server::Server, switch::Switch};

mod data;
mod raft_channel;
mod raft_request;
mod raft_server;
mod raft_socket;
mod server;
mod state;
fn main() {
    let switch: Arc<Switch<u32>> = Arc::new(Switch::new());
    let servers = (0..5)
        .map(|id| Server::new(id, switch.clone()))
        .map(|server| Arc::new(task::block_on(server)))
        .collect::<Vec<_>>();
    for server in servers {
        Server::init(server);
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
