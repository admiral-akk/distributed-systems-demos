use std::{
    collections::HashSet,
    time::{Duration, SystemTime},
};

use async_std::{sync::Arc, sync::Mutex, task};
use rand::Rng;

use crate::{
    raft_channel::RaftChannel,
    raft_request::{RaftRequest, RequestType},
    raft_socket::RaftSocket,
};

#[derive(PartialEq, Default, Debug)]
pub enum RaftState {
    #[default]
    Offline,
    Follower {},
    Candidate {
        votes: HashSet<u32>,
    },
    Leader,
}

// This state is written to disc (or somewhere else safe) before any request is sent out.
#[derive(Default, Debug)]
pub struct PersistentState {
    current_term: u32,
    voted_for: Option<u32>,
}

const HEARTBEAT_LENGTH: Duration = Duration::from_millis(2000);

#[derive(Debug)]
pub struct ServerState {
    state: RaftState,
    last_heartbeat: SystemTime,
    persistent_state: PersistentState,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            state: Default::default(),
            last_heartbeat: SystemTime::now(),
            persistent_state: Default::default(),
        }
    }
}

pub struct RaftServer {
    pub state: Arc<Mutex<ServerState>>,
    pub socket: Arc<Mutex<RaftSocket>>,
    pub channel: Arc<Mutex<RaftChannel>>,
}

pub enum RaftResponse {
    Target { id: u32, response: RaftRequest },
    Broadcast { response: RaftRequest },
}

impl RaftServer {
    pub fn new(id: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(ServerState::default())),
            socket: Arc::new(Mutex::new(RaftSocket::new(id))),
            channel: Arc::new(Mutex::new(RaftChannel::new(id))),
        }
    }
    pub async fn heartbeat(state: Arc<Mutex<ServerState>>, channel: Arc<Mutex<RaftChannel>>) {
        loop {
            let rand_sleep =
                rand::thread_rng().gen_range((HEARTBEAT_LENGTH / 2)..(2 * HEARTBEAT_LENGTH));
            task::sleep(rand_sleep).await;
            {
                let mut state = state.lock().await;
                if SystemTime::now()
                    .duration_since(state.last_heartbeat)
                    .unwrap()
                    .cmp(&HEARTBEAT_LENGTH)
                    .is_le()
                {
                    continue;
                }
                let channel = channel.lock().await;
                state.state = RaftState::Candidate {
                    votes: HashSet::from([channel.id]),
                };
                state.persistent_state.voted_for = Some(channel.id);
                state.persistent_state.current_term += 1;
                let result = channel
                    .broadcast(RaftRequest {
                        term: state.persistent_state.current_term,
                        sender: 0,
                        request: RequestType::Vote {},
                    })
                    .await;
            }
        }
    }

    pub async fn handle(
        state: Arc<Mutex<ServerState>>,
        channel: Arc<Mutex<RaftChannel>>,
        socket: Arc<Mutex<RaftSocket>>,
    ) {
        loop {
            let request;
            {
                request = socket.lock().await.reciever.recv().await;
            }
            if let Ok(request) = request {
                let mut state = state.lock().await;
                let channel = channel.lock().await;
                println!(
                    "Recieved request: {:?}, state: {:?}, reciever: {}",
                    request, state, channel.id
                );
                let response = state.handle(request, channel.server_count(), SystemTime::now());
                if let Some(response) = response {
                    match response {
                        RaftResponse::Target { id, response } => channel.send(id, response).await,
                        RaftResponse::Broadcast { response } => channel.broadcast(response).await,
                    };
                }
            }
        }
    }

    pub async fn leader_heartbeat(
        state: Arc<Mutex<ServerState>>,
        channel: Arc<Mutex<RaftChannel>>,
    ) {
        loop {
            task::sleep(201 * HEARTBEAT_LENGTH / 3210).await;
            {
                let mut state = state.lock().await;
                if state.state == RaftState::Leader {
                    state.last_heartbeat = SystemTime::now();
                    let channel = channel.lock().await;
                    channel
                        .broadcast(RaftRequest {
                            term: state.persistent_state.current_term,
                            sender: 0,
                            request: RequestType::Append {},
                        })
                        .await;
                }
            }
        }
    }

    pub async fn start(&mut self) {
        {
            let mut state = self.state.lock().await;
            state.state = RaftState::Follower {};
        }
        task::spawn(RaftServer::heartbeat(
            self.state.clone(),
            self.channel.clone(),
        ));
        task::spawn(RaftServer::leader_heartbeat(
            self.state.clone(),
            self.channel.clone(),
        ));
        task::spawn(RaftServer::handle(
            self.state.clone(),
            self.channel.clone(),
            self.socket.clone(),
        ))
        .await;
    }
}

impl ServerState {
    pub fn handle(
        &mut self,
        request: RaftRequest,
        server_count: usize,
        time: SystemTime,
    ) -> Option<RaftResponse> {
        if request.term > self.persistent_state.current_term {
            self.state = RaftState::Follower {};
            self.last_heartbeat = SystemTime::now();
            self.persistent_state.voted_for = None;
            self.persistent_state.current_term = request.term;
        }

        match &mut self.state {
            RaftState::Follower {} => {
                match request.request {
                    RequestType::Append {} => {
                        // If the term is behind, we have a leader from the last round.
                        let success = request.term >= self.persistent_state.current_term;

                        if success {
                            // Reset the heartbeat, since our dear leader is alive.
                            self.last_heartbeat = time;
                        }

                        return Some(RaftResponse::Target {
                            id: request.sender,
                            response: RaftRequest {
                                term: self.persistent_state.current_term,
                                sender: 0,
                                request: RequestType::AppendResponse { success },
                            },
                        });
                    }
                    RequestType::Vote {} => {
                        // If the request is from a previous term, then we vote no.
                        let mut vote = request.term >= self.persistent_state.current_term;

                        // If we've already voted, and the requester is a different person, then vote no.
                        if let Some(previous_vote) = self.persistent_state.voted_for {
                            vote &= previous_vote == request.sender;
                        }
                        if vote {
                            self.persistent_state.voted_for = Some(request.sender);
                        }

                        return Some(RaftResponse::Target {
                            id: request.sender,
                            response: RaftRequest {
                                term: self.persistent_state.current_term,
                                sender: 0,
                                request: RequestType::VoteResponse { vote },
                            },
                        });
                    }
                    // Followers don't send Append/Vote, and so shouldn't handle responses.
                    _ => None,
                }
            }
            // Candidates only care about getting votes. If they get a majority, they transform into leaders.
            RaftState::Candidate { votes } => {
                match request.request {
                    RequestType::VoteResponse { vote } => {
                        if vote {
                            println!("Recieved vote!");
                            votes.insert(request.sender);
                        }
                        if votes.len() > server_count / 2 {
                            self.persistent_state.current_term += 1;
                            self.state = RaftState::Leader;
                            self.last_heartbeat = SystemTime::now();
                            return Some(RaftResponse::Broadcast {
                                response: RaftRequest {
                                    term: self.persistent_state.current_term,
                                    sender: 0,
                                    request: RequestType::Append {},
                                },
                            });
                        } else {
                            return None;
                        }
                    }
                    // Followers don't send Append/Vote, and so shouldn't handle responses.
                    _ => None,
                }
            }
            // Leaders just care that their commands are sucessful. If they ever fail, they get sad and quit their job.
            RaftState::Leader => {
                match request.request {
                    RequestType::AppendResponse { success } => {
                        if !success {
                            self.state = RaftState::Follower {};
                            self.last_heartbeat = SystemTime::now();
                            self.persistent_state.voted_for = None;
                        }
                        return None;
                    }
                    // Followers don't send Append/Vote, and so shouldn't handle responses.
                    _ => None,
                }
            }

            _ => None,
        }
    }
}
