use std::env;
use std::fs::read_to_string;
use std::mem::transmute;
use vonneumann::ExecutableMemory;
use std::io::Write;

const COMMANDS: [char; 8] = ['+','-','>','<','.',',','[',']'];
const MAX_MEM: usize = 65535;
const OFFSET: usize = MAX_MEM/2;

#[derive(Debug, Clone, Copy)]
enum CMD {
    Plus(u8),
    Minus(u8),
    PtrR(u32),
    PtrL(u32),
    Push(u16),
    Pull,
    JmpR,
    JmpL,
}

fn get_32bit_offset(jump_from: usize, jump_to: usize) -> u32 {
    if jump_to >= jump_from {
        let diff = jump_to - jump_from;
        // println!("1: {}", diff as i64);
        return diff as u32;
    } else {
        let diff = jump_from - jump_to;
        let diff_unsigned = diff as u32;
        // println!("2: {}", diff_unsigned as i64);
        return !diff_unsigned.wrapping_sub(1);
    }
}

struct Buff {
    data: Vec<u8>,
    jmp_stack: Vec<usize>,
}

#[allow(dead_code)]
impl<'a> Buff {
    fn push(&mut self, v: u8) {
        self.data.push(v);
    }
    fn append(&mut self, vec: Vec<u8>) {
        for v in vec {
            self.push(v)
        }
    }
    fn stack(&mut self, v: usize) {
        self.jmp_stack.push(v);
    }
    fn len(&self) -> usize {
        self.data.len()
    }
    fn u32(&mut self, value: u32) {
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes);
    }
    fn u64(&mut self, value: u64) {
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes);
    }
    fn replace_u64(&mut self, value: u64, index: usize) {
        // index -= 1;
        let bytes = value.to_le_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            self.data[index + i] = *byte;
        }
    }
    fn replace_u32(&mut self, value: u32, index: usize) {
        let bytes = value.to_le_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if let Some(elem) = self.data.get_mut(index + i) {
                *elem = byte;
            }
        }
    }
    fn encode(&mut self, cmds: Vec<CMD>, pointer: u64) {
        // mov [program mem ptr] to r13
        self.push(0x49);
        self.push(0xbd);
        self.u64(pointer); // pointer to program memory
        for cmd in cmds {
            match cmd {
                CMD::Plus(n) => {
                    // increment the value r13 points at by n (8bit)
                    self.append(vec![0x41, 0x80, 0x45, 0x00, n]);
                },
                CMD::Minus(n) => {
                    // deincrement the value r13 points at by n (8bit)
                    self.append(vec![0x41, 0x80, 0x6D, 0x00, n]);
                },
                CMD::PtrR(n) => {
                    // increment r13 by n (8bit)
                    if n <= 255 {
                        self.append(vec![0x49, 0x83, 0xC5, n as u8]);
                    } else {
                        let bytes = n.to_le_bytes();
                        self.append(vec![0x49, 0x81, 0xC5, bytes[0], bytes[1], bytes[2], bytes[3]]);
                    }
                },
                CMD::PtrL(n) => {
                    // increment r13 by n (8bit)
                    if n <= 255 {
                        self.append(vec![0x49, 0x83, 0xED, n as u8]);
                    } else {
                        let bytes = n.to_le_bytes();
                        self.append(vec![0x49, 0x81, 0xED, bytes[0], bytes[1], bytes[2], bytes[3]]);
                    }
                },
                CMD::Push(n) => {
                    self.append(vec![0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]);
                    self.append(vec![0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00]);
                    self.append(vec![0x4C, 0x89, 0xEE]);
                    self.append(vec![0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00]);
                    // once registers are setup, we can repeat our systemcall if desired
                    for _ in 0..n {
                        self.append(vec![0x0F, 0x05]);
                    }
                },
                CMD::Pull => {
                    self.append(vec![0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00]);
                    self.append(vec![0x48, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00]);
                    self.append(vec![0x4C, 0x89, 0xEE]);
                    self.append(vec![0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00]);
                    self.append(vec![0x0F, 0x05]);
                },
                CMD::JmpR => {
                    self.append(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);
                    // append the current position to stack
                    self.stack(self.len());
                    self.append(vec![0x0F, 0x84]);
                    // append placeholder jump location
                    self.u32(0_u32);
                },
                CMD::JmpL => {
                    // get the location of the most recent JmpR
                    match self.jmp_stack.pop() {
                        None => {
                            eprintln!("Mismatched brackets @: {}", self.len());
                            break
                        },
                        Some(open_offset) => {
                            self.append(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);
                            // calc value to jump to
                            let jmp_bk_from = self.len() + 6;
                            let jmp_bk_to = open_offset + 6;
                            let rel_jmp_bk_offset = get_32bit_offset(jmp_bk_from, jmp_bk_to);
                            // append it
                            self.append(vec![0x0F, 0x85]);
                            self.u32(rel_jmp_bk_offset);
                            // calculate value jump back from
                            let jmp_fw_from = open_offset + 6;
                            let jmp_fw_to = self.len();
                            let rel_jmp_fw_offset = get_32bit_offset(jmp_fw_from, jmp_fw_to);
                            // overwrite the placeholder value with it
                            self.replace_u32(rel_jmp_fw_offset, open_offset + 2);
                        },
                    }
                },
            }
        }
        // return to caller
        // 4c 89 e8
        self.append(vec![0x4c, 0x89, 0xe8]);
        self.push(0xc3);
    }
}

// attempts to read the first arg as file to string
// will Panic! if the file doesn't exist or cannot be read
fn read_input_file(arg: String) -> String {
    // let args: Vec<String> = env::args().collect(); // get command line arguments

    // if args.len() != 2 {
    //     println!("Incorrect number of args.");
    //     eprintln!("Usage: {} <filename>", args[0]);
    //     exit(1)
    // }

    // let filepath = &args[1]; // the last argument is the file name

    match read_to_string(arg) {
        Ok(content) => content,
        Err(e) => panic!("Failed to read file, err: {}", e),
    }
}

fn parse(input: String) -> Vec<CMD> {
    let mut chars: Vec<char> = input.chars().collect();
    chars.retain(|&c| COMMANDS.contains(&c));
    let mut map: Vec<CMD> = vec![];
    chars.push(' '); // prevents out of bounds err
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '+' => {
                let mut l = 0;
                while chars[i] == '+' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Plus(l));
            },
            '-' => {
                let mut l = 0;
                while chars[i] == '-' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Minus(l));
            },
            '>' => {
                let mut l = 0;
                while chars[i] == '>' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrR(l));
            },
            '<' => {
                let mut l = 0;
                while chars[i] == '<' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrL(l));
            },
            '.' => {
                let mut l = 0;
                while chars[i] == '.' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Push(l));
            },
            ',' => {
                map.push(CMD::Pull);
                i += 1;
            },
            '[' => {
                map.push(CMD::JmpR);
                i += 1;
            },
            ']' => {
                map.push(CMD::JmpL);
                i += 1;
            },
            _ => {
                i += 1;
            },
        }
    }
    map
}

#[allow(dead_code)]
fn show_hex_32(bytes: &Vec<u8>) {
    let mut ct = 0;
    for byte in bytes {
        print!("{:02X} ", byte);
        ct += 1;
        if ct % 16 == 0 {
            println!("")
        }
    }
    if !(ct % 16 == 0) {
        println!("")
    }
}

#[allow(dead_code)]
fn show_hex_64(bytes: &Vec<u8>) {
    let mut ct = 0;
    for byte in bytes {
        print!("{:02X} ", byte);
        ct += 1;
        if ct % 32 == 0 {
            println!("")
        }
    }
    if !(ct % 32 == 0) {
        println!("")
    }
}

// build & run CST
fn run(code: Vec<CMD>) {
    // this struct will create and store our code from the CST
    let mut buffer = Buff {
        data: vec![], // where our code is stored
        jmp_stack: vec![], // used for tracking jmp offsets
    };

    {
        // allocate runtime mem
        let mem: [u8; MAX_MEM] = [0; MAX_MEM];
        let pointer: u64 = mem.as_ptr().wrapping_add(MAX_MEM/2) as u64;
        buffer.encode(code, pointer);
        // show_hex_32(&buffer.data);
        // copy our program to executable memory
        let program = ExecutableMemory::with_contents(&buffer.data);
        unsafe {
            let f = transmute::<*mut u8, unsafe fn()>(program.as_ptr());
            f();
        }
    }
}

// build & run CST with external ptr
fn run_with_ptr(code: Vec<CMD>, pointer: u64) -> ExecutableMemory {
    // this struct will create and store our code from the CST
    let mut buffer = Buff {
        data: vec![], // where our code is stored
        jmp_stack: vec![], // used for tracking jmp offsets
    };

    buffer.encode(code, pointer);
    // show_hex_32(&buffer.data);
    // copy our program to executable memory
    let program = ExecutableMemory::with_contents(&buffer.data);
    return program
    // unsafe {
    //     let f = transmute::<*mut u8, unsafe fn()>(program.as_ptr());
    //     f();
    // }
}

fn main() {
    if env::args().count() > 1 {
        let args: Vec<String> = env::args().skip(1).collect(); // get command line arguments
        for arg in args {
            let txt = read_input_file(arg);
            let code = parse(txt);
            run(code);
        }
    } else {
        let mut mem = [0u8; MAX_MEM];
        let mut pos = OFFSET;
        loop {
            print!("\n>>> ");
            std::io::stdout().flush().expect("Err");
            let mut raw_input = String::new();
            std::io::stdin().read_line(&mut raw_input).unwrap();
            let input: String = raw_input.to_lowercase();
            let str_input: &str = &input;
            match &str_input {
                _ if str_input.starts_with("q") => {
                    break
                }
                _ if str_input.starts_with("print") => {
                    if let Some(start) = str_input.find('(') {
                        if let Some(end) = str_input.find(')') {
                            let args_str = &str_input[start + 1..end];
                            let args: Vec<usize> = args_str.split(',')
                                .map(|s| s.trim().parse::<usize>().expect("Invalid number"))
                                .collect();
                            for i in args[0]..args[1] {
                                print!("{:?} ", mem[OFFSET+i]);
                            }
                        }
                    }
                }
                _ if str_input.starts_with("p") => {
                    for i in 0..32 {
                        print!("{:?} ", mem[OFFSET+i]);
                    }
                }
                _ => {
                    let cst = parse(input);
                    if cst.len() > 0 {
                        let eph_mem = mem;
                        let base_ptr = eph_mem.as_ptr() as usize;
                        let program = run_with_ptr(cst, eph_mem.as_ptr().wrapping_add(pos) as u64);
                        unsafe {
                            let f = transmute::<*mut u8, unsafe fn() -> u64>(program.as_ptr());
                            let new_pos = f() as usize;
                            pos = new_pos-base_ptr;
                        }
                        mem = eph_mem;
                    }
                }
            }
        }
    }
}

