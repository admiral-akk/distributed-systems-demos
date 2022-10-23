use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    time::{Duration, SystemTime},
};

use async_std::{sync::Arc, sync::Mutex, task};
use futures::FutureExt;
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
    Follower {
        volitile: VolitileState,
    },
    Candidate {
        volitile: VolitileState,
        votes: HashSet<u32>,
    },
    Leader {
        leader_volitile: VolitileLeaderState,
        volitile: VolitileState,
    },
}

#[derive(PartialEq, Default, Debug)]
pub struct VolitileState {
    pub commit_index: usize,
    pub last_applied: usize,
}

impl VolitileState {
    pub fn leader_start_state(&self, followers: Vec<u32>) -> VolitileLeaderState {
        VolitileLeaderState {
            next_index: followers
                .iter()
                .map(|id| (*id, self.last_applied + 1))
                .collect(),
            match_index: followers.iter().map(|id| (*id, 0)).collect(),
        }
    }
}

#[derive(PartialEq, Default, Debug)]
pub struct VolitileLeaderState {
    pub next_index: HashMap<u32, usize>,
    pub match_index: HashMap<u32, usize>,
}

#[derive(Default, Debug)]
struct LogEntry<DataType> {
    data: DataType,
    term: u32,
}

// This state is written to disc (or somewhere else safe) before any request is sent out.
#[derive(Default, Debug)]
pub struct PersistentState<DataType> {
    current_term: u32,
    voted_for: Option<u32>,
    log: Vec<LogEntry<DataType>>,
}

impl<DataType> PersistentState<DataType> {
    pub fn volitile_start_state(&self) -> VolitileState {
        VolitileState::default()
    }
}

const HEARTBEAT_LENGTH: Duration = Duration::from_millis(2000);

#[derive(Debug)]
pub struct ServerState<DataType> {
    state: RaftState,
    last_heartbeat: SystemTime,
    persistent_state: PersistentState<DataType>,
}

impl<DataType> Default for ServerState<DataType> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            last_heartbeat: SystemTime::now(),
            persistent_state: PersistentState {
                current_term: 0,
                voted_for: None,
                log: Vec::new(),
            },
        }
    }
}

pub struct RaftServer<DataType> {
    pub state: Arc<Mutex<ServerState<DataType>>>,
    pub socket: Arc<Mutex<RaftSocket>>,
    pub channel: Arc<Mutex<RaftChannel>>,
}

pub enum RaftResponse {
    Target { id: u32, response: RaftRequest },
    Broadcast { response: RaftRequest },
}

impl<DataType: Debug + Send + Sync + 'static> RaftServer<DataType> {
    pub fn new(id: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(ServerState::default())),
            socket: Arc::new(Mutex::new(RaftSocket::new(id))),
            channel: Arc::new(Mutex::new(RaftChannel::new(id))),
        }
    }
    pub async fn heartbeat(
        state: Arc<Mutex<ServerState<DataType>>>,
        channel: Arc<Mutex<RaftChannel>>,
    ) {
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
                    volitile: state.persistent_state.volitile_start_state(),
                };
                state.persistent_state.voted_for = Some(channel.id);
                state.persistent_state.current_term += 1;
                channel
                    .broadcast(RaftRequest {
                        term: state.persistent_state.current_term,
                        sender: 0,
                        request: RequestType::Vote {
                            log_length: state.persistent_state.log.len(),
                            log_term: match state.persistent_state.log.last() {
                                Some(entry) => entry.term,
                                None => 0,
                            },
                        },
                    })
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn kill_leader(state: Arc<Mutex<ServerState<DataType>>>) {
        loop {
            let rand_sleep =
                rand::thread_rng().gen_range((3 * HEARTBEAT_LENGTH)..(5 * HEARTBEAT_LENGTH));
            task::sleep(rand_sleep).await;
            {
                let mut state = state.lock().await;
                state.state = RaftState::Offline;
            }
            let rand_sleep =
                rand::thread_rng().gen_range((HEARTBEAT_LENGTH)..(2 * HEARTBEAT_LENGTH));
            task::sleep(rand_sleep).await;
            {
                let mut state = state.lock().await;
                state.state = RaftState::Follower {
                    volitile: state.persistent_state.volitile_start_state(),
                };
            }
        }
    }

    pub async fn handle(
        state: Arc<Mutex<ServerState<DataType>>>,
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
                let response = state.handle(request, channel.servers(), SystemTime::now());
                if let Some(response) = response {
                    match response {
                        RaftResponse::Target { id, response } => {
                            channel.send(id, response).await;
                        }
                        RaftResponse::Broadcast { response } => {
                            channel.broadcast(response).await;
                        }
                    };
                }
            }
        }
    }

    pub async fn leader_heartbeat(
        state: Arc<Mutex<ServerState<DataType>>>,
        channel: Arc<Mutex<RaftChannel>>,
    ) {
        loop {
            task::sleep(201 * HEARTBEAT_LENGTH / 3210).await;
            {
                let mut state = state.lock().await;
                match &state.state {
                    RaftState::Leader {
                        leader_volitile,
                        volitile,
                    } => {
                        state.last_heartbeat = SystemTime::now();
                        let channel = channel.lock().await;
                        channel
                            .broadcast(RaftRequest {
                                term: state.persistent_state.current_term,
                                sender: 0,
                                request: RequestType::Append {},
                            })
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }
    }

    pub async fn start(&mut self) {
        {
            self.state
                .lock()
                .then(|mut f| async move {
                    f.state = RaftState::Follower {
                        volitile: f.persistent_state.volitile_start_state(),
                    };
                })
                .await;
        }
        task::spawn(RaftServer::heartbeat(
            self.state.clone(),
            self.channel.clone(),
        ));
        task::spawn(RaftServer::leader_heartbeat(
            self.state.clone(),
            self.channel.clone(),
        ));
        task::spawn(RaftServer::kill_leader(self.state.clone()));
        task::spawn(RaftServer::handle(
            self.state.clone(),
            self.channel.clone(),
            self.socket.clone(),
        ))
        .await;
    }
}

impl<DataType> ServerState<DataType> {
    pub fn handle(
        &mut self,
        request: RaftRequest,
        servers: Vec<u32>,
        time: SystemTime,
    ) -> Option<RaftResponse> {
        if request.term > self.persistent_state.current_term {
            self.state = RaftState::Follower {
                volitile: self.persistent_state.volitile_start_state(),
            };
            self.last_heartbeat = SystemTime::now();
            self.persistent_state.voted_for = None;
            self.persistent_state.current_term = request.term;
        }

        match &mut self.state {
            RaftState::Follower { volitile } => {
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
                    RequestType::Vote {
                        log_length,
                        log_term,
                    } => {
                        // If the request is from a previous term, then we vote no.
                        let mut vote = request.term >= self.persistent_state.current_term;

                        // If we've committed any values, then we can ask these questions.
                        if !self.persistent_state.log.is_empty() {
                            // If the candidate's log ends with an older term, then we reject them.
                            vote &= self.persistent_state.log.last().unwrap().term <= log_term;
                            if self.persistent_state.log.last().unwrap().term == log_term {
                                // If the candidate's log is shorter and in the same term, then reject them.
                                vote &= self.persistent_state.log.len() <= log_length;
                            }
                        }

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
            RaftState::Candidate { volitile, votes } => {
                match request.request {
                    RequestType::VoteResponse { vote } => {
                        if vote {
                            println!("Recieved vote!");
                            votes.insert(request.sender);
                        }
                        if votes.len() > (servers.len() + 1) / 2 {
                            self.persistent_state.current_term += 1;
                            let volitile = self.persistent_state.volitile_start_state();
                            self.state = RaftState::Leader {
                                leader_volitile: volitile.leader_start_state(servers),
                                volitile,
                            };
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
            RaftState::Leader {
                leader_volitile,
                volitile,
            } => {
                match request.request {
                    RequestType::AppendResponse { success } => {
                        if !success {
                            self.state = RaftState::Follower {
                                volitile: self.persistent_state.volitile_start_state(),
                            };
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
