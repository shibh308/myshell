use std::collections::{BTreeSet, HashMap};

pub struct TrieNode {
    pub l: usize,
    pub r: usize,
    pub children: HashMap<u8, usize>,
}

pub struct Trie {
    pub texts: Vec<String>,
    nodes: Vec<TrieNode>,
    idx: Option<usize>,
}

impl Trie {
    pub fn new(texts: Vec<String>) -> Trie {
        let mut byte_texts = texts
            .iter()
            .map(|x| x.as_bytes())
            .collect::<BTreeSet<_>>()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let texts = byte_texts
            .iter()
            .map(|x| String::from_utf8(x.iter().cloned().collect::<Vec<_>>()).unwrap())
            .collect::<Vec<_>>();
        let n = texts.len();
        let mut nodes = Vec::new();
        Self::create(0, n, 0, 0, &byte_texts, &mut nodes);
        Trie {
            texts,
            nodes,
            idx: Some(0),
        }
    }
    fn create(
        l: usize,
        r: usize,
        d: usize,
        id: usize,
        texts: &Vec<&[u8]>,
        nodes: &mut Vec<TrieNode>,
    ) -> usize {
        nodes.push(TrieNode {
            l,
            r,
            children: HashMap::new(),
        });
        let mut now_id = id + 1;
        let mut dat: Option<(usize, u8)> = None;
        let mut children = HashMap::new();
        for i in l..=r {
            if i != r && texts[i].len() <= d {
                continue;
            }
            if let Some(dat_) = dat {
                if i == r || dat_.1 != texts[i][d] {
                    children.insert(dat_.1, now_id);
                    now_id = Self::create(dat_.0, i, d + 1, now_id, texts, nodes);
                    if i < r {
                        dat = Some((i, texts[i][d]));
                    }
                }
            } else if i < r {
                dat = Some((i, texts[i][d]));
            }
        }
        nodes[id].children = children;
        now_id
    }
    pub fn reset(&mut self) {
        self.idx = Some(0);
    }
    pub fn search(&mut self, c: char) {
        for c in [c].iter().collect::<String>().as_bytes() {
            self.idx = if let Some(i) = self.idx {
                self.nodes[i].children.get(c).cloned()
            } else {
                None
            };
        }
    }
    pub fn get_range(&self) -> std::ops::Range<usize> {
        match self.idx {
            Some(i) => self.nodes[i].l..self.nodes[i].r,
            None => 0..0,
        }
    }
}
