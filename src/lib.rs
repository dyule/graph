extern crate rustc_serialize;
use std::collections::{HashMap, HashSet, BTreeMap};
use rustc_serialize::{Decoder as BaseDecoder, Decodable};
use rustc_serialize::json::{Json, ToJson, Decoder};
use std::fs::File;
use std::io::{Read, Write};
use std::io::Result as IOResult;
use std::string::ToString;

#[derive (RustcDecodable, Debug, PartialEq)]
pub struct Node {
    pub id: i32,
    pub props:HashMap<String, String>,
}

#[derive (RustcDecodable, Debug, PartialEq)]
pub struct Edge {
    pub labels: Vec<String>
}

#[derive (Debug)]
pub struct Graph {
    nodes: HashMap<i32, Node>,
    edges: HashMap<i32, HashMap<i32, Edge>>,
    reverse_edges: HashMap<i32, HashSet<i32>>,
    max_node_id: i32
}


impl PartialEq for Graph {
    fn eq(&self, other:&Self) -> bool {
        return self.nodes.eq(&other.nodes) && self.edges.eq(&other.edges)
    }
}

/// Used to convert a Node to JSON
impl ToJson for Node {
	fn to_json(&self) -> Json {
		let mut d = BTreeMap::new();
		let mut props = BTreeMap::new();
		d.insert("id".to_string(), self.id.to_json());
		for (k, v) in self.props.iter() {
			props.insert(k.clone(), Json::String(v.clone()));
		}
		d.insert("props".to_string(), Json::Object(props));
		Json::Object(d)
	}
}

impl ToJson for Edge {
	fn to_json(&self) -> Json {
		let mut d = BTreeMap::new();
		d.insert("labels".to_string(), Json::Array(self.labels.iter().map(|l| Json::String(l.to_string())).collect()));
		Json::Object(d)
	}
}

impl ToJson for Graph {
	fn to_json(&self) -> Json {
		let mut d = BTreeMap::new();
		let mut edge_json_map = BTreeMap::new();
		d.insert("nodes".to_string(), Json::Object(self.nodes.iter().map(|(i, n)| (i.to_string(), n.to_json())).collect()));
		for (index, edge) in self.edges.iter() {
			edge_json_map.insert(index.to_string(), Json::Object(edge.iter().map(|(i, n)| (i.to_string(), n.to_json())).collect()));
		}
		d.insert("edges".to_string(), Json::Object(edge_json_map));
		Json::Object(d)
    }
}
impl Decodable for Graph {

    fn decode<D:BaseDecoder>(decoder: &mut D) -> Result<Self, D::Error> {
        decoder.read_struct("root", 0, |decoder| {
            let mut max_node_id = 0;
            let nodes = try!(decoder.read_struct_field("nodes", 0, |decoder| {
                decoder.read_map(|decoder, len| {
                    let mut nodes = HashMap::new();
                    for idx in 0..len {
                        let node_index = try!(decoder.read_map_elt_key(idx, |decoder| decoder.read_i32()));
                        let node:Node = try!(decoder.read_map_elt_val(idx, |decoder| Decodable::decode(decoder)));
                        if node_index > max_node_id {
                            max_node_id = node_index + 1;
                        }
                        nodes.insert(node_index, node);
                    }
                    Ok(nodes)
                })

        }));
        let mut g = Graph{
                max_node_id: max_node_id,
                nodes: nodes,
                edges: HashMap::new(),
                reverse_edges: HashMap::new()
            };
            try!(decoder.read_struct_field("edges", 0, |decoder| {
                decoder.read_map(|decoder, len| {
                    for idx in 0..len {
                        let source_index = try!(decoder.read_map_elt_key(idx, |decoder| decoder.read_i32()));
                        let edge_map:HashMap<i32, Edge> = try!(decoder.read_map_elt_val(idx, |decoder| Decodable::decode(decoder)));
                        for (destination_index, edge) in edge_map.iter() {
                            g.connect_nodes(source_index, *destination_index, edge.labels.clone())
                        };
                    }
                    Ok(())


                })
            }));

            Ok(g)
        })

    }
}

impl Node {

}

impl Graph {
    fn new() -> Graph {
        Graph {
            max_node_id: 0,
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new()
        }
    }

    fn get_node_next_id(&mut self) -> i32 {
        self.max_node_id = self.max_node_id + 1;
        self.max_node_id
    }

    fn add_node(&mut self) -> i32 {
        let id = self.get_node_next_id();
        let node:Node = Node {
            id: id,
            props: HashMap::new(),
        };
        self.nodes.insert(id, node);
        id
    }

    fn remove_node(&mut self, node_id:i32) {
        if !self.nodes.contains_key(&node_id) {
            panic!("Tried to remove a node that didn't exist: {}", node_id);
        }
        if let Some(re) = self.reverse_edges.get(&node_id) {
            for n in re {
                self.edges.get_mut(n).unwrap().remove(&node_id);
            }
        }
        if let Some(e) = self.edges.get(&node_id) {
            for n in e.keys() {
                self.reverse_edges.get_mut(n).unwrap().remove(&node_id);
            }
        }
        self.edges.remove(&node_id);
        self.reverse_edges.remove(&node_id);
        self.nodes.remove(&node_id);
    }

    fn get_node(&self, node_id:i32) -> Option<&Node> {
        self.nodes.get(&node_id)
    }

    fn get_node_mut(&mut self, node_id:i32) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }

    fn connect_nodes(&mut self, origin:i32, destination:i32, labels:Vec<String>) {

        if !self.nodes.contains_key(&origin) {
            panic!("Tried to connect node id that wasn't in the database: {}", origin)
        }

        if !self.nodes.contains_key(&destination) {
            panic!("Tried to connect node id that wasn't in the database: {}", destination)
        }
        let e = Edge {
            labels: labels
        };
        if !self.edges.contains_key(&origin) {
            self.edges.insert(origin, HashMap::new());
        }
        self.edges.get_mut(&origin).unwrap().insert(destination, e);
        if match self.reverse_edges.get_mut(&origin){
            Some(m) => {
                if m.contains(&destination) {
                    m.remove(&destination);
                    false
                }
                else {
                    true
                }
            },
            None => true
        } {
            if !self.reverse_edges.contains_key(&destination) {
                self.reverse_edges.insert(destination, HashSet::new());
            }
            self.reverse_edges.get_mut(&destination).unwrap().insert(origin);
        }
    }

    fn are_connected(&mut self, origin:i32, destination:i32) -> bool {
        if !self.nodes.contains_key(&origin) {
            panic!("Tried to check node id that wasn't in the database: {}", origin)
        }

        if !self.nodes.contains_key(&destination) {
            panic!("Tried to check node id that wasn't in the database: {}", destination)
        }
        match self.edges.get(&origin) {
            Some(m) => m.contains_key(&destination),
            None    => false
        }
    }

    /// Encode and decode from file

    pub fn from_json(json: Json) -> Graph {
        let mut decoder = Decoder::new(json);
		match Decodable::decode(&mut decoder) {
			Ok(x) => x,
			Err(e) => panic!("Could not decode to graph: {}", e)
		}
    }

	pub fn read_from_file(name: String) -> Graph {
		let mut contents = String::new();
		let mut file:File = File::open(name).unwrap();

		file.read_to_string(&mut contents).unwrap();
		let jsonstring = match Json::from_str(&contents) {
			Ok(a) => a,
			Err(e) => panic!("Error reading JSON string: {}", e)
		};

		Graph::from_json(jsonstring)
	}

	pub fn write_to_file(&self, name: &'static str) -> IOResult<usize> {
		let mut file = try!(File::create(name));
		file.write(self.to_json().to_string().as_bytes())
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_serialize::json::{Json, ToJson};
    #[test]
    fn adding_nodes() {
        let mut g = Graph::new();
        let id1 = g.add_node();
        let id2 = g.add_node();
        {
            let n1 = g.get_node(id1).unwrap();
            assert!(n1.id == id1);
        }
        {
            let n2 = g.get_node_mut(id2).unwrap();
            assert!(n2.id == id2);
            n2.props.insert("hey".to_string(), "you".to_string());
        }

    }

    #[test]
    fn connecting_nodes() {
        let mut g = Graph::new();
        let id1 = g.add_node();
        let id2 = g.add_node();
        let id3 = g.add_node();
        g.connect_nodes(id1, id2, vec!["hello".to_string(), "hi".to_string()]);
        assert!(g.are_connected(id1, id2));
        assert!(!g.are_connected(id2, id1));
        g.connect_nodes(id2, id1, vec![]);
        g.connect_nodes(id2, id3, vec![]);

        assert!(g.are_connected(id1, id2));
        assert!(g.are_connected(id2, id1));
        assert!(g.are_connected(id2, id3));
        assert!(!g.are_connected(id3, id2));
    }

    #[test]
    fn removing_nodes() {
        let mut g = Graph::new();
        let id1 = g.add_node();
        let id2 = g.add_node();
        let id3 = g.add_node();
        g.connect_nodes(id1, id2, vec!["hello".to_string(), "hi".to_string()]);
        assert!(g.are_connected(id1, id2));
        assert!(!g.are_connected(id2, id1));
        g.connect_nodes(id2, id1, vec![]);
        g.connect_nodes(id2, id3, vec![]);

        assert!(g.are_connected(id1, id2));
        assert!(g.are_connected(id2, id1));
        assert!(g.are_connected(id2, id3));
        assert!(!g.are_connected(id3, id2));
        assert!(!g.are_connected(id3, id1));
        assert!(!g.are_connected(id1, id3));

        g.remove_node(id3);
        assert!(g.are_connected(id1, id2));
        assert!(g.are_connected(id2, id1));
        assert!(g.get_node(id3).is_none());
    }

    #[test]
    fn json_io() {
        let mut g = Graph::new();
        let id1 = g.add_node();
        let id2 = g.add_node();
        let id3 = g.add_node();
        g.connect_nodes(id1, id2, vec!["hello".to_string(), "hi".to_string()]);
        g.connect_nodes(id2, id1, vec![]);
        g.connect_nodes(id2, id3, vec![]);

        let json_string = g.to_json().to_string();
        let expected_string = r#"{"edges":{"1":{"2":{"labels":["hello","hi"]}},"2":{"1":{"labels":[]},"3":{"labels":[]}}},"nodes":{"1":{"id":1,"props":{}},"2":{"id":2,"props":{}},"3":{"id":3,"props":{}}}}"#;
        assert!(json_string == expected_string);
        let new_json = Json::from_str(expected_string).unwrap();
        let g2 = Graph::from_json(new_json);
        assert!(g.eq(&g2));
    }
}
