use std::collections::HashMap;

use crate::{Queue, State, Token};
use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};

macro_rules! push_edx {
    ($ops: expr) => {
        dynasm!($ops
            ; mov rcx, rdi
            ; mov rax, QWORD State::push as _
            ; call rax
        );
    };
}

macro_rules! pop_to_eax {
    ($ops: expr) => {
        dynasm!($ops
            ; mov rcx, rdi
            ; mov rax, QWORD State::must_pop as _
            ; call rax
        );
    };
}

extern "win64" fn print(state: &mut State) {
    println!("{}", state.must_pop());
}

extern "win64" fn pow(state: &mut State) {
    let b = state.must_pop();
    let a = state.must_pop();
    state.push(a.pow(b as u32));
}

extern "win64" fn quit() {
    std::process::exit(0);
}

extern "win64" fn custom_def_fallback(_state: &mut State, name: *const u8, name_len: usize) {
    let name = unsafe { std::slice::from_raw_parts(name, name_len) };
    let name = std::str::from_utf8(name).unwrap();
    eprintln!("Custom def: {} not compiled.", name);
    eprintln!("In future, this will fallback to the interpreter.");
    std::process::exit(1);
}

type Code = extern "win64" fn(&mut State);

pub struct DefsMap(HashMap<String, *mut *const u8>);

impl Default for DefsMap {
    fn default() -> Self {
        Self::new()
    }
}

impl DefsMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn reserve(&mut self, name: String) {
        let pointer = Box::leak(Box::new(custom_def_fallback as *const u8));
        self.0.insert(name, pointer);
    }

    pub fn fill(&mut self, name: &str, code: Code) {
        if !self.0.contains_key(name) {
            self.reserve(name.to_string());
        }

        let pointer = self.0.get(name).unwrap();
        let pointer_to_code = code as *const u8;
        unsafe {
            **pointer = pointer_to_code;
        }
    }

    pub fn get_first(&self, name: &str) -> *mut *const u8 {
        *self.0.get(name).unwrap()
    }

    pub fn get_first_or_reserve(&mut self, name: String) -> *mut *const u8 {
        if !self.0.contains_key(&name) {
            self.reserve(name.clone());
        }
        self.get_first(&name)
    }

    pub fn get_second(&self, name: &str) -> Option<Code> {
        // unsafe { *(*self.0.get(name).unwrap() as *const Code) }
        if let Some(pointer) = self.0.get(name) {
            let pointer = *pointer as *const Code;
            Some(unsafe { *pointer })
        } else {
            None
        }
    }
}

pub fn compile(queue: Queue<Token>, defs: &mut DefsMap) -> extern "win64" fn(&mut State) {
    use Token::*;

    let mut ops = dynasmrt::x64::Assembler::new().unwrap();

    let entry = ops.offset();

    // Prelude
    dynasm!(ops
        ; .arch x64
        ; push rbp
        ; mov rbp, rsp
        ; sub rsp, BYTE 16 // Some multiple of 16

        // state* is passed in rcx
        // move it to a non-volatile register: rdi
        ; mov rdi, rcx
    );

    // Stores the offset before each token
    let mut addr_table = Vec::<*const u8>::with_capacity(queue.len() + 1);

    // Fill it with dummy values
    for _ in 0..queue.len() {
        addr_table.push(std::ptr::null());
    }
    addr_table.push(std::ptr::null());

    let addr_table = addr_table.leak();

    let mut offsets = vec![];

    let addr_table_ptr = addr_table.as_ptr();

    // Codegen
    for (i, token) in queue.iter().enumerate() {
        offsets.push(ops.offset());
        match token {
            Num(x) => {
                dynasm!(ops
                    ; mov edx, DWORD *x
                );
                push_edx!(ops);
            }
            Add => {
                // Pop 2, add them, push the result
                pop_to_eax!(ops);
                dynasm!(ops
                    // Push to stack
                    ; push rax
                    ; sub rsp, BYTE 8 // For 16-byte alignment
                );
                pop_to_eax!(ops);
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rdx
                    ; add edx, eax // edx = edx + eax
                );
                push_edx!(ops);
            }
            Sub => {
                // Pop 2, subtract them, push the result
                pop_to_eax!(ops);
                dynasm!(ops
                    // Push to stack
                    ; push rax
                    ; sub rsp, BYTE 8 // For 16-byte alignment
                );
                pop_to_eax!(ops);
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rdx
                    ; sub eax, edx // eax = eax - edx
                    ; mov edx, eax
                );
                push_edx!(ops);
            }
            Mul => {
                // Pop 2, multiply them, push the result
                pop_to_eax!(ops);
                dynasm!(ops
                    // Push to stack
                    ; push rax
                    ; sub rsp, BYTE 8 // For 16-byte alignment
                );
                pop_to_eax!(ops);
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rdx
                    ; imul edx, eax // edx = edx * eax
                );
                push_edx!(ops);
            }
            Div => {
                // a b / => a / b
                pop_to_eax!(ops); // Pop b
                dynasm!(ops
                    ; push rax
                    ; sub rsp, BYTE 8
                );
                pop_to_eax!(ops); // Pop a
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rbx // ebx = b
                    ; cdq
                    ; idiv ebx // eax = eax / ebx = a / b
                    ; mov edx, eax
                );
                push_edx!(ops);
            }
            Mod => {
                // a b % => a % b
                pop_to_eax!(ops); // Pop b
                dynasm!(ops
                    ; push rax
                    ; sub rsp, BYTE 8
                );
                pop_to_eax!(ops); // Pop a
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rbx // ebx = b
                    ; cdq
                    ; idiv ebx // eax = eax % ebx = a % b
                    ; mov edx, edx
                );
                push_edx!(ops);
            }
            Pow => {
                dynasm!(ops
                    ; mov rcx, rdi
                    ; mov rax, QWORD pow as _
                    ; call rax
                );
            }
            Drop => {
                pop_to_eax!(ops);
            }
            Swap => {
                // a b swap => b a
                pop_to_eax!(ops);
                dynasm!(ops
                    ; push rax
                    ; sub rsp, BYTE 8
                );
                pop_to_eax!(ops);
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rdx
                    ; push rax
                    ; sub rsp, BYTE 8
                );
                push_edx!(ops);
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rdx
                );
                push_edx!(ops);
            }
            Rot => {
                // We are going to use R13, R14
                dynasm!(ops
                    ; push r13
                    ; push r14
                );
                // a b c rot => b c a
                pop_to_eax!(ops); // c
                dynasm!(ops
                    ; mov r13, rax // r13 = c
                );
                pop_to_eax!(ops); // b
                dynasm!(ops
                    ; mov r14, rax // r14 = b
                );
                // The stack is now: b, c
                pop_to_eax!(ops); // a
                dynasm!(ops
                    ; mov rdx, r14 // rax = b
                    ; mov r14, rax // r14 = a
                );
                push_edx!(ops); // Push b
                dynasm!(ops
                    ; mov rdx, r13 // rax = c
                );
                push_edx!(ops); // Push c
                dynasm!(ops
                    ; mov rdx, r14 // rax = a
                );
                push_edx!(ops); // Push a
                dynasm!(ops
                    ; pop r14
                    ; pop r13
                );
            }
            Less => {
                // a b < => 1 if a < b else 0
                pop_to_eax!(ops); // Pop b
                dynasm!(ops
                    ; push rax
                    ; sub rsp, BYTE 8
                );
                pop_to_eax!(ops); // Pop a
                dynasm!(ops
                    ; add rsp, BYTE 8
                    ; pop rbx // ebx = b
                    ; cmp eax, ebx
                    ; setl al
                    ; movzx edx, al // edx = 1 if a < b else 0
                );
                push_edx!(ops);
            }
            Pick => {
                // n pick
                // Push the n-th element in the stack
                pop_to_eax!(ops); // eax = n
                dynasm!(ops
                    ; mov edx, eax // edx = n
                    ; mov rcx, rdi
                    ; mov rax, QWORD State::must_pick as _
                    ; call rax
                    ; mov edx, eax
                );
                push_edx!(ops);
            }
            Skip => {
                // n skip
                // Jump to n+i+1 th address in the table
                let j = i + 1;
                pop_to_eax!(ops); // eax = n
                dynasm!(ops
                    ; mov edx, DWORD j as _ // edx = i+1
                    ; add eax, edx // eax = n + i + 1
                    ; mov rdx, QWORD addr_table_ptr as _
                    ; mov rdx, [rdx + rax * 8] // rdx = addr_table[n+i]
                    ; jmp rdx
                );
            }
            If => {
                // cond if a b c
                // if cond == 0: Jump to i+4 th
                let j = i + 4;
                pop_to_eax!(ops); // cond
                dynasm!(ops
                    ; test eax, eax
                    ; jnz >non_zero
                    ; mov edx, DWORD j as _ // edx = i+4
                    ; mov rcx, QWORD addr_table_ptr as _
                    ; mov rcx, [rcx + rdx * 8] // rcx = addr_table[i+4]
                    ; jmp rcx
                    ;non_zero:
                );
            }
            Print => {
                dynasm!(ops
                    ; mov rcx, rdi
                    ; mov rax, QWORD print as _
                    ; call rax
                );
            }
            Quit => {
                dynasm!(ops
                    ; mov rax, QWORD quit as _
                    ; call rax
                );
            }
            DefBegin | DefEnd => {
                panic!("Can not compile definition tokens");
            }
            Custom(name) => {
                let first_pointer = defs.get_first_or_reserve(name.clone());
                // Leak the name to make it live long enough
                let ptr_name = name.clone().into_bytes().leak();
                let ptr_name = ptr_name.as_ptr();

                // Call (*pointer) with state, name, name_len
                dynasm!(ops
                    ; mov rcx, rdi
                    ; mov rdx, QWORD ptr_name as _
                    ; mov r8, QWORD name.len() as _
                    ; mov rax, QWORD first_pointer as _
                    ; mov rax, [rax]
                    ; call rax
                );
            }
        }
    }

    let end = ops.offset();
    dynasm!(ops
    // Epilogue
        ; leave
        ; ret
    );

    let buf = ops.finalize().unwrap();

    // We here need to leak buf to make it live long enough
    let buf = Box::leak(Box::new(buf));

    // Fill address table
    for (i, off) in offsets.into_iter().enumerate() {
        addr_table[i] = buf.ptr(off);
    }
    addr_table[queue.len()] = buf.ptr(end);

    unsafe { std::mem::transmute(buf.ptr(entry)) }
}
