use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphCommand {
    AddNode {
        node_type: String,
        kind: NodeKind,
        position: (f32, f32),
    },
    RemoveNode {
        id: u64,
    },
    Connect {
        from: u64,
        from_port: usize,
        to: u64,
        to_port: usize,
    },
    Disconnect {
        from: u64,
        to: u64,
    },
    SetParameter {
        node_id: u64,
        param_name: String,
        value: f32,
    },
    SetNodePosition {
        id: u64,
        position: (f32, f32),
    },
    Play,
    Pause,
    Stop,
    SaveGraph(String),
    LoadGraph(String),
}

/// The kind of node in the audio graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Generator,
    Filter,
    Sink,
}
