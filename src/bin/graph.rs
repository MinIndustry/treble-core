use petgraph::algo::toposort;
use petgraph::graph::Graph;

struct Node {
    pub value: &'static str,
}

fn main() {
    println!("Building a graph");
    let mut graph = Graph::new();

    let node_a = Node { value: "a" };
    let node_b = Node { value: "b" };
    let node_c = Node { value: "c" };
    let node_d = Node { value: "d" };
    let node_e = Node { value: "e" };
    let node_f = Node { value: "f" };
    let node_g = Node { value: "g" };

    let a = graph.add_node(&node_a);
    let b = graph.add_node(&node_b);
    let c = graph.add_node(&node_c);
    let d = graph.add_node(&node_d);
    let e = graph.add_node(&node_e);
    let f = graph.add_node(&node_f);
    let g = graph.add_node(&node_g);

    graph.add_edge(a, b, ());
    graph.add_edge(a, c, ());
    graph.add_edge(b, d, ());
    graph.add_edge(c, d, ());
    graph.add_edge(d, e, ());
    graph.add_edge(e, f, ());
    graph.add_edge(f, g, ());

    if let Ok(sorted) = toposort(&graph, None) {
        for node in sorted {
            print!("{:?} - ", graph[node].value);
        }
    }
    println!();
}
