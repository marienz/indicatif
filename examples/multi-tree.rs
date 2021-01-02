use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        Self(since_the_epoch)
    }

    fn gen_range(&mut self, n: u64) -> u64 {
        let mut state = self.0;
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(2685821657736338717u64);
        self.0 = state;
        state % n
    }
}

#[derive(Debug, Clone)]
enum Action {
    AddProgressBar(usize),
    IncProgressBar(usize),
}

#[derive(Clone, Debug)]
struct Elem {
    key: String,
    index: usize,
    indent: usize,
    progress_bar: ProgressBar,
}

lazy_static! {
    static ref ELEMENTS: [Elem; 9] = [
        Elem { indent: 1, index: 0, progress_bar: ProgressBar::new(32), key: "jumps".to_string() },
        Elem { indent: 2, index: 1, progress_bar: ProgressBar::new(32), key: "lazy".to_string() },
        Elem { indent: 0, index: 0, progress_bar: ProgressBar::new(32), key: "the".to_string() },
        Elem { indent: 3, index: 3, progress_bar: ProgressBar::new(32), key: "dog".to_string() },
        Elem { indent: 2, index: 2, progress_bar: ProgressBar::new(32), key: "over".to_string() },
        Elem { indent: 2, index: 1, progress_bar: ProgressBar::new(32), key: "brown".to_string() },
        Elem { indent: 1, index: 1, progress_bar: ProgressBar::new(32), key: "quick".to_string() },
        Elem { indent: 3, index: 5, progress_bar: ProgressBar::new(32), key: "a".to_string() },
        Elem { indent: 3, index: 3, progress_bar: ProgressBar::new(32), key: "fox".to_string() },
    ];
}

fn main() {
    let mp = Arc::new(MultiProgress::new());
    let sty_main = ProgressStyle::default_bar().template("{bar:40.green/yellow} {pos:>4}/{len:4}");
    let sty_aux = ProgressStyle::default_bar().template("{spinner:.green} {msg} {pos:>4}/{len:4}");

    let pb_main = mp.add(ProgressBar::new(
        ELEMENTS.iter().map(|e| e.progress_bar.length()).sum(),
    ));
    pb_main.set_style(sty_main);
    for elem in ELEMENTS.iter() {
        elem.progress_bar.set_style(sty_aux.clone());
    }

    let tree: Arc<Mutex<Vec<&Elem>>> = Arc::new(Mutex::new(Vec::with_capacity(ELEMENTS.len())));
    let tree2 = Arc::clone(&tree);

    let mp2 = Arc::clone(&mp);
    let _ = thread::spawn(move || {
        mp2.println("Getting started...");
        let mut rng = Rng::new();
        pb_main.tick();
        loop {
            match get_action(&mut rng, &tree) {
                None => {
                    // all elements were exhausted
                    pb_main.finish();
                    return;
                }
                Some(Action::AddProgressBar(el_idx)) => {
                    mp2.println("Adding more bars...");
                    let elem = &ELEMENTS[el_idx];
                    let pb = mp2.insert(elem.index + 1, elem.progress_bar.clone());
                    pb.set_message(&format!("{}  {}", "  ".repeat(elem.indent), elem.key));
                    tree.lock().unwrap().insert(elem.index, &elem);
                }
                Some(Action::IncProgressBar(el_idx)) => {
                    let elem = &tree.lock().unwrap()[el_idx];
                    elem.progress_bar.inc(1);
                    let pos = elem.progress_bar.position();
                    let len = elem.progress_bar.length();
                    if pos >= len {
                        elem.progress_bar.finish_with_message(&format!(
                            "{}{} {}",
                            "  ".repeat(elem.indent),
                            "✔",
                            elem.key
                        ));
                    }
                    pb_main.inc(1);
                }
            }
            thread::sleep(Duration::from_millis(15));
        }
    });

    mp.join().unwrap();

    // Simulate a message being added after the last progress bar finishes.
    // buffered_messages() is intended to retrieve such messages sent from another thread
    // (tricky to write a realistic example for).
    mp.println("All done!");
    mp.add(ProgressBar::new(1)).println("Yes, really!");
    for msg in mp.buffered_messages() {
        println!("{}", msg);
    }

    println!("===============================");
    println!("the tree should be the same as:");
    for elem in tree2.lock().unwrap().iter() {
        println!("{}  {}", "  ".repeat(elem.indent), elem.key);
    }
}

fn get_action<'a>(rng: &'a mut Rng, tree: &Mutex<Vec<&Elem>>) -> Option<Action> {
    let elem_len = ELEMENTS.len() as u64;
    let list_len = tree.lock().unwrap().len() as u64;
    let sum_free = tree
        .lock()
        .unwrap()
        .iter()
        .map(|e| {
            let pos = e.progress_bar.position();
            let len = e.progress_bar.length();
            len - pos
        })
        .sum::<u64>();
    if sum_free == 0 && list_len == elem_len {
        // nothing to do more
        return None;
    } else if sum_free == 0 && list_len < elem_len {
        // there is no place to make an increment
        return Some(Action::AddProgressBar(tree.lock().unwrap().len()));
    } else {
        loop {
            let list = tree.lock().unwrap();
            let k = rng.gen_range(17);
            if k == 0 && list_len < elem_len {
                return Some(Action::AddProgressBar(list.len()));
            } else {
                let l = (k % list_len) as usize;
                let pos = list[l].progress_bar.position();
                let len = list[l].progress_bar.length();
                if pos < len {
                    return Some(Action::IncProgressBar(l));
                }
            }
        }
    }
}
