mod defs;
pub mod jit;
use std::collections::HashMap;

pub use defs::*;

pub struct State {
    defs: HashMap<String, Queue<Token>>,
    jitted: jit::DefsMap,
    return_stack: ReturnStack,
    stack: TheStack,
    pub queue: TheQueue,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            defs: HashMap::new(),
            jitted: jit::DefsMap::new(),
            return_stack: ReturnStack::new(),
            stack: TheStack::new(),
            queue: TheQueue::new(),
        }
    }

    fn is_end(&self) -> bool {
        self.queue.is_empty() && self.return_stack.is_empty()
    }

    extern "win64" fn must_pop(&mut self) -> i32 {
        // println!("Popping {:?}", &self.stack as *const _);
        self.stack.pop().unwrap_or_else(|| {
            eprintln!("Stack underflow");
            std::process::exit(1);
        })
    }

    extern "win64" fn push(&mut self, value: i32) {
        // let ptr = &mut self.stack as *mut _;
        // println!("Pushing {} to {:?}", value, ptr);
        self.stack.push(value);
    }

    extern "win64" fn must_pick(&self, n: usize) -> i32 {
        if let Some(v) = self.stack.pick(n - 1) {
            *v
        } else {
            eprintln!("Index out of bounds");
            std::process::exit(1);
        }
    }

    fn must_pop_queue(&mut self) -> Token {
        self.queue.pop().unwrap_or_else(|| {
            eprintln!("Queue underflow");
            std::process::exit(1);
        })
    }

    pub fn print_stack(&self) {
        print!("Stack: ");
        for n in self.stack.iter() {
            print!("{} ", n);
        }
        println!();
    }

    pub fn parse(&mut self, input: &str) {
        let mut parsed = parse(input);
        while let Some(token) = parsed.pop() {
            self.queue.push(token);
        }
    }

    fn after_return(&mut self) {
        // return stack should not be empty
        self.queue = self.return_stack.pop().unwrap();
    }
}

fn parse(input: &str) -> TheQueue {
    let mut queue = TheQueue::new();
    for token in input.split_whitespace() {
        match token {
            "+" => queue.push(Token::Add),
            "-" => queue.push(Token::Sub),
            "*" => queue.push(Token::Mul),
            "/" => queue.push(Token::Div),
            "%" => queue.push(Token::Mod),
            "**" => queue.push(Token::Pow),
            "<" => queue.push(Token::Less),
            ":" => queue.push(Token::DefBegin),
            ";" => queue.push(Token::DefEnd),
            "if" => queue.push(Token::If),
            "skip" => queue.push(Token::Skip),
            "print" => queue.push(Token::Print),
            "quit" => queue.push(Token::Quit),
            "swap" => queue.push(Token::Swap),
            "rot" => queue.push(Token::Rot),
            "pick" => queue.push(Token::Pick),
            "drop" => queue.push(Token::Drop),
            _ => {
                if let Ok(num) = token.parse::<i32>() {
                    queue.push(Token::Num(num))
                } else {
                    queue.push(Token::Custom(token.to_string()))
                }
            }
        };
    }
    queue
}

macro_rules !error {
    ($($arg:tt)*) => {
        {
            eprintln!($($arg)*);
            std::process::exit(1);
        }
    }
}

/// Clac intrepreter
pub fn eval(state: &mut State, jit: bool) {
    use Token::*;
    while !state.is_end() {
        if state.queue.is_empty() {
            state.after_return();
            if state.queue.is_empty() {
                continue;
            }
        }

        let token = state.queue.pop().unwrap();

        match token {
            Add => {
                let a = state.must_pop();
                let b = state.must_pop();
                state.stack.push(a + b);
            }
            Sub => {
                let a = state.must_pop();
                let b = state.must_pop();
                state.stack.push(b - a);
            }
            Mul => {
                let a = state.must_pop();
                let b = state.must_pop();
                state.stack.push(a * b);
            }
            Div => {
                let a = state.must_pop();
                let b = state.must_pop();

                if a == 0 {
                    error!("Division by zero");
                }
                if a == -1 && b == i32::MIN {
                    error!("Overflow");
                }

                state.stack.push(b / a);
            }
            Mod => {
                let a = state.must_pop();
                let b = state.must_pop();

                if a == 0 {
                    error!("Division by zero");
                }
                if a == -1 && b == i32::MIN {
                    error!("Overflow");
                }

                state.stack.push(b % a);
            }
            Pow => {
                let a = state.must_pop();
                let b = state.must_pop();
                if a < 0 {
                    error!("Negative exponent");
                }
                state.stack.push(b.pow(a as u32));
            }
            Less => {
                let a = state.must_pop();
                let b = state.must_pop();
                state.stack.push(if b < a { 1 } else { 0 });
            }
            Num(num) => state.stack.push(num),
            Swap => {
                let a = state.must_pop();
                let b = state.must_pop();
                state.stack.push(a);
                state.stack.push(b);
            }
            Rot => {
                let a = state.must_pop();
                let b = state.must_pop();
                let c = state.must_pop();
                state.stack.push(b);
                state.stack.push(a);
                state.stack.push(c);
            }
            Pick => {
                // Use iter
                let n = state.must_pop();
                if n <= 0 {
                    error!("Invalid index");
                }
                let n = n as usize;
                state.stack.push(state.must_pick(n));
            }
            If => {
                let cond = state.must_pop();
                if cond == 0 {
                    // Skip next three
                    for _ in 0..3 {
                        state.must_pop_queue();
                    }
                }
            }
            Skip => {
                let n = state.must_pop();
                if n < 0 {
                    error!("Negative skip");
                }
                for _ in 0..n {
                    state.must_pop_queue();
                }
            }
            Print => {
                let n = state.must_pop();
                println!("{}", n);
            }
            Drop => {
                state.must_pop();
            }
            Quit => std::process::exit(0),
            DefBegin => {
                let mut def = Queue::new();

                loop {
                    let token = state.must_pop_queue();
                    if token == DefEnd {
                        break;
                    }
                    def.push(token);
                }

                if def.is_empty() {
                    error!("Empty definition");
                }
                if let Custom(name) = &def.pop().unwrap() {
                    if jit {
                        println!("Compiling {}...", name);
                        let code = jit::compile(def, &mut state.jitted);
                        state.jitted.fill(name, code);
                    } else {
                        state.defs.insert(name.clone(), def);
                    }
                    println!("Defined {}", name);
                } else {
                    error!("Invalid definition");
                }
            }
            DefEnd => error!("Unexpected definition end"),
            Custom(name) => {
                if let Some(def) = state.defs.get(&name) {
                    // Move the queue to the return stack
                    state.return_stack.push(state.queue.take());
                    state.queue.become_iter(def);
                } else {
                    if jit {
                        if let Some(code) = state.jitted.get_second(&name) {
                            code(state);
                            continue;
                        }
                    }
                    error!("Unknown definition: {}", name);
                }
            }
        }
    }
}

// Quick eval: give a queue and get the stack
// pub fn quick_eval(queue: TheQueue) -> TheStack {
//     let mut state = State::new();
//     state.queue = queue;
//     eval(&mut state);
//     state.stack
// }
