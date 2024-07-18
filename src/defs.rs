use std::collections::{LinkedList, VecDeque};

// pub struct Stack<T>(LinkedList<T>);
pub struct Stack<T>(Vec<T>);

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, value: T) {
        self.0.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Pick the nth element from the top of the stack
    pub fn pick(&self, n: usize) -> Option<&T> {
        // self.0.iter().rev().nth(n)
        self.0.get(self.0.len() - n - 1)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }
}

// pub struct Queue<T>(LinkedList<T>);
pub struct Queue<T>(VecDeque<T>);

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }
    pub fn push(&mut self, value: T) {
        self.0.push_back(value);
    }
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop_front()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.0.iter()
    }
}

pub enum TheQueue {
    Real(Queue<Token>),
    Iter(std::collections::vec_deque::Iter<'static, Token>),
    None, // This should only be temp
}

impl TheQueue {
    pub fn new() -> Self {
        Self::Real(Queue::new())
    }

    pub fn push(&mut self, value: Token) {
        match self {
            Self::Real(queue) => queue.push(value),
            _ => unreachable!(),
        }
    }

    pub fn pop(&mut self) -> Option<Token> {
        match self {
            Self::Real(queue) => queue.pop(),
            Self::Iter(iter) => {
                let token = iter.next();
                token.cloned()
            }
            Self::None => unreachable!(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Real(queue) => queue.is_empty(),
            Self::Iter(iter) => iter.len() == 0,
            Self::None => unreachable!(),
        }
    }

    pub fn take(&mut self) -> TheQueue {
        std::mem::replace(self, Self::None)
    }

    pub fn unwrap(self) -> Queue<Token> {
        match self {
            Self::Real(queue) => queue,
            _ => panic!(),
        }
    }

    pub fn become_iter(&mut self, queue: &Queue<Token>) {
        let _ = std::mem::replace(
            self,
            Self::Iter(unsafe { std::mem::transmute(queue.iter()) }),
        );
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Token {
    Num(i32),
    Add,  // +
    Sub,  // -
    Mul,  // *
    Div,  // /
    Mod,  // %
    Pow,  // **
    Less, // <

    // Definitions
    DefBegin, // :
    DefEnd,   // ;

    // Controls
    If,   // if
    Skip, // skip

    // Other
    Print, // print
    Quit,  // quit
    Swap,  // swap
    Rot,   // rot
    Pick,  // pick
    Drop,  // drop

    Custom(String),
}

// Custom abbrs
pub type TheStack = Stack<i32>;
pub type ReturnStack = Stack<TheQueue>;
