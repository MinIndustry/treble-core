use serde::{Deserialize, Serialize};

/// All graph-related commands: structural mutations + playback control.
///
/// Structural variants (AddNode, RemoveNode, Connect, Disconnect) mutate
/// `GraphData` in the command thread only.
///
/// Playback variants (Play, Pause, Stop, SetParameter) also send
/// `AudioMessage`s to the render thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphCommand {
    // -- Structural (command-thread only) --
    AddNode {
        id: u64,
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

    // -- Playback control (command-thread → render-thread) --
    StartNode {
        id: u64,
    },
    StopNode {
        id: u64,
    },
    SetParameter {
        node_id: u64,
        param_name: String,
        value: f32,
    },
}

/// The kind of node in the audio graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Generator,
    Filter,
    Sink,
}
