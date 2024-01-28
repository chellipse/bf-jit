use std::env;
use std::fs::read_to_string;
use std::process::exit;
use std::mem::transmute;
use vonneumann::ExecutableMemory;

const PAGE_SIZE: usize = 4096;
const COMMANDS: [char; 8] = ['+','-','>','<','.',',','[',']'];
const MAX_MEM: usize = 32768;
// const MAX_MEM: usize = 32;

#[derive(Debug, Clone, Copy)]
enum CMD {
    Plus(u8),
    Minus(u8),
    PtrR(u8),
    PtrL(u8),
    Push(u8),
    Pull,
    JmpR,
    JmpL,
}

fn get_32bit_offset(jump_from: usize, jump_to: usize) -> u32 {
    // dbg!(jump_from, jump_to);
    if jump_to >= jump_from {
        let diff = jump_to - jump_from;
        // dbg!(diff);
        // dbg!(diff as u32);
        // assert!(diff < (1u64 << 31));
        return diff as u32;
    } else {
        // Here the diff is negative, so we need to encode it as 2's complement.
        let diff = jump_from - jump_to;
        // dbg!(diff);
        // dbg!(diff as u32);
        // println!("WRAP: {}", !(diff as u32).wrapping_sub(1) as i32);
        // dbg!(!(diff as u32).wrapping_sub(1));
        // assert!(diff - 1 < (1u64 << 31));
        let diff_unsigned = diff as u32;
        return !diff_unsigned.wrapping_sub(1);
    }
}

struct Buff {
    data: Vec<u8>,
    stack: Vec<usize>,
}

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
        self.stack.push(v);
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
    fn replace_u32(&mut self, value: u32, index: usize) {
        let bytes = value.to_le_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if let Some(elem) = self.data.get_mut(index + i) {
                *elem = byte;
            }
        }
    }
    fn encode(&mut self, cmds: Vec<CMD>) {
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
                    self.append(vec![0x49, 0x83, 0xC5, n]);
                },
                CMD::PtrL(n) => {
                    // increment r13 by n (8bit)
                    self.append(vec![0x49, 0x83, 0xED, n]);
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
                    self.stack(self.len());
                    self.append(vec![0x0F, 0x84]);
                    self.u32(0_u32);
                },
                CMD::JmpL => {
                    match self.stack.pop() {
                        None => {
                            eprintln!("Mismatched brackets @: {}", self.data.len());
                            break
                        },
                        Some(open_offset) => {
                            // println!("{:?}", open_offset);
                            self.append(vec![0x41, 0x80, 0x7D, 0x00, 0x00]);
                            // get offset for jmp back
                            let jmp_bk_from = self.len() + 6;
                            let jmp_bk_to = open_offset + 6;
                            let rel_jmp_bk_offset = get_32bit_offset(jmp_bk_from, jmp_bk_to);
                            // make jmp
                            self.append(vec![0x0F, 0x85]);
                            self.u32(rel_jmp_bk_offset);
                            // dbg!(rel_jmp_bk_offset as i32);
                            // get offset for jmp forward
                            let jmp_fw_from = open_offset + 6;
                            let jmp_fw_to = self.len();
                            let rel_jmp_fw_offset = get_32bit_offset(jmp_fw_from, jmp_fw_to);
                            // dbg!(rel_jmp_fw_offset);
                            self.replace_u32(rel_jmp_fw_offset, open_offset + 2);
                            // dbg!(self.data[open_offset + 2]);
                            // dbg!(self.data[open_offset + 3]);
                            // dbg!(self.data[open_offset + 4]);
                            // dbg!(self.data[open_offset + 5]);
                        },
                    }
                },
            }
        }
    }
}

// attempts to read the first arg as file to string
// will Panic! if the file doesn't exist or cannot be read
fn read_input_file() -> String {
    let args: Vec<String> = env::args().collect(); // get command line arguments

    if args.len() != 2 {
        println!("Incorrect number of args.");
        eprintln!("Usage: {} <filename>", args[0]);
        exit(1)
    }

    let filepath = &args[1]; // the last argument is the file name

    match read_to_string(filepath) {
        Ok(content) => content,
        Err(e) => panic!("Failed to read file, err: {}", e),
    }
}

fn parse(code: &mut Vec<char>) -> Vec<CMD> {
    let mut map: Vec<CMD> = vec![];
    code.push(' ');
    let mut i = 0;
    while i < code.len() {
        match code[i] {
            '+' => {
                let mut l = 0;
                while code[i] == '+' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Plus(l));
            },
            '-' => {
                let mut l = 0;
                while code[i] == '-' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Minus(l));
            },
            '>' => {
                let mut l = 0;
                while code[i] == '>' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrR(l));
            },
            '<' => {
                let mut l = 0;
                while code[i] == '<' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrL(l));
            },
            '.' => {
                let mut l = 0;
                while code[i] == '.' {
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

fn main() {
    // read first arg as file to string
    let data: String = read_input_file();

    let mut code_txt: Vec<char> = data.chars()
                              .collect();
    // filter code_txt
    code_txt.retain(|&c| COMMANDS.contains(&c));

    // parse into CST
    let parsed_code = parse(&mut code_txt);

    // init runtime mem
    let mem: [u8; MAX_MEM] = [0; MAX_MEM];

    let mut buffer = Buff {
        data: vec![], // where our code is stored
        stack: vec![], // used for tracking jmp offsets
    };

    // mov [program mem ptr] to r13
    buffer.push(0x49);
    buffer.push(0xbd);
    buffer.u64(mem.as_ptr() as u64);

    buffer.encode(parsed_code);

    buffer.push(0xc3);
    // dbg!(mem.as_ptr());
    // println!("buffer len: {:?}", buffer.len());
    let len = buffer.len();

    // dbg!(len / PAGE_SIZE + 1);
    // let mut code = ExecutableMemory::new(
    //     len / PAGE_SIZE + 1
    // );
    // code.as_slice_mut()[..buffer.len()].copy_from_slice(&buffer.data);
    let code = ExecutableMemory::with_contents(&buffer.data);

    // println!("PROGRAM CODE: ");
    // show_hex_64(&buffer.data);

    let rows = (len-(len% 32)) / 32;
    let extra = len%32;
    println!("PROGRAM LEN: {}*64 + {}", rows, extra);

    let time = std::time::Instant::now();
    println!("--- START ---");
    let mark1 = time.elapsed().as_micros();
    unsafe {
        let f = transmute::<*mut u8, unsafe fn()>(code.as_ptr());
        f();
    }
    let mark2 = time.elapsed().as_micros();
    println!("\n--- END ---");

    let diff = mark2-mark1;
    println!("PROGRAM RUNTIME: {}s {}ms {}us", (diff/1000/1000), (diff/1000%1000), diff%1000);
    // println!("PROGRAM MEM: {:?}", mem);
    // println!("{:?}", code);
}

