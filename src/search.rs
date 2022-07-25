use std::collections::{BTreeSet, HashMap};

pub struct TrieNode {
    pub l: usize,
    pub r: usize,
    pub end: bool,
    pub max_idx: usize,
    pub children: HashMap<u8, usize>,
}

pub struct Trie {
    pub texts: Vec<String>,
    counts: Vec<usize>,
    nodes: Vec<TrieNode>,
    idx: Option<usize>,
}

impl Trie {
    pub fn new(texts: Vec<String>, hist: &Vec<(i32, String)>) -> Trie {
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
        let mut trie = Trie {
            texts,
            nodes,
            counts: vec![0; n],
            idx: Some(0),
        };
        trie.read_history(&hist);
        trie
    }
    fn read_history(&mut self, hist: &Vec<(i32, String)>) {
        for (_, cmd) in hist {
            if let Some(head) = cmd.split_ascii_whitespace().next() {
                for c in head.chars() {
                    self.search(c);
                }
                if let Some(idx) = self.idx {
                    if self.nodes[idx].end {
                        self.counts[self.nodes[idx].l] += 1;
                    }
                }
                self.reset();
            }
        }
        for node in &mut self.nodes {
            let op = (node.l..node.r).max_by(|x, y| self.counts[*x].cmp(&self.counts[*y]));
            node.max_idx = op.unwrap();
        }
    }
    pub fn add_cnt(&mut self, cmd: &String) {
        for c in cmd.chars() {
            self.search(c);
        }
        let res = self.idx;
        self.reset();
        if let Some(idx) = res {
            if !self.nodes[idx].end {
                return;
            }
            let idx = self.nodes[idx].l;
            self.counts[idx] += 1;
            for c in cmd.chars() {
                let maxi = self.counts[self.nodes[self.idx.unwrap()].max_idx];
                if self.counts[idx] > maxi {
                    self.nodes[self.idx.unwrap()].max_idx = idx;
                }
                self.search(c);
            }
            let maxi = self.counts[self.nodes[self.idx.unwrap()].max_idx];
            if self.counts[idx] > maxi {
                self.nodes[self.idx.unwrap()].max_idx = idx;
            }
            self.reset();
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
            end: false,
            max_idx: l,
            children: HashMap::new(),
        });
        let mut now_id = id + 1;
        let mut dat: Option<(usize, u8)> = None;
        let mut children = HashMap::new();
        for i in l..=r {
            if i != r && texts[i].len() <= d {
                nodes[id].end = true;
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
    pub fn get_max(&self) -> Option<usize> {
        match self.idx {
            Some(i) => Some(self.nodes[i].max_idx),
            None => None,
        }
    }
    pub fn get_range(&self) -> std::ops::Range<usize> {
        match self.idx {
            Some(i) => self.nodes[i].l..self.nodes[i].r,
            None => 0..0,
        }
    }
}
