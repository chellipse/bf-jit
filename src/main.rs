use std::env;
use std::fs::read_to_string;
use std::process::exit;
use std::collections::HashMap;

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

fn jmp_look_right(mut ip: usize, code: &Vec<CMD>) -> usize {
    let mut brack_ct = 1;
    // println!("IP: {}", ip);
    ip += 1;
    while brack_ct > 0 && ip < code.len() && ip > 0 {
        match code[ip] {
            CMD::JmpL(_) => {
                brack_ct -= 1;
                if brack_ct == 0 {
                    break
                }
                ip += 1;
            },
            CMD::JmpR(_) => {
                brack_ct += 1;
                ip += 1;
            },
            _ => {
                ip += 1;
            },
        }
    }
    // println!("BLR: {}", ip);
    ip
}

fn jmp_look_left(mut ip: usize, code: &Vec<CMD>) -> usize {
    let mut brack_ct = 1;
    ip -= 1;
    while brack_ct > 0 && ip < code.len() && ip > 0 {
        match code[ip] {
            CMD::JmpL(_) => {
                brack_ct += 1;
                ip -= 1;
            },
            CMD::JmpR(_) => {
                brack_ct -= 1;
                if brack_ct == 0 {
                    break
                }
                ip -= 1;
            },
            _ => {
                ip -= 1;
            },
        }
    }
    // println!("BLL: {}", ip);
    ip
}

fn add_jmp_values(mut code: Vec<CMD>) -> Vec<CMD> {
    let copy = code.clone();
    for (index, c) in code.iter_mut().enumerate() {
        match c {
            CMD::JmpR(val) => {
                let value = jmp_look_right(index, &copy);
                *val = value;
            }
            CMD::JmpL(val) => {
                let value = jmp_look_left(index, &copy);
                *val = value;
            }
            _ => {}
        }
    }
    code
}

fn parse(code: &mut Vec<char>) -> Vec<CMD> {
    let mut map: Vec<CMD> = vec![];

    code.push(' ');
    let mut i = 0;
    while i < code.len() {
        match code[i] {
            '+' => {
                // println!("C: {}, I: {}", code[i], i);
                let mut l = 0;
                while code[i] == '+' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Plus(l));
            },
            '-' => {
                // println!("C: {}", code[i]);
                let mut l = 0;
                while code[i] == '-' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Minus(l));
            },
            '>' => {
                // println!("C: {}", code[i]);
                let mut l = 0;
                while code[i] == '>' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrR(l));
            },
            '<' => {
                // println!("C: {}", code[i]);
                let mut l = 0;
                while code[i] == '<' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::PtrL(l));
            },
            '.' => {
                // println!("C: {}", code[i]);
                let mut l = 0;
                while code[i] == '.' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Push(l));
            },
            ',' => {
                // println!("C: {}", code[i]);
                let mut l = 0;
                while code[i] == ',' {
                    i += 1;
                    l += 1;
                }
                map.push(CMD::Pull(l));
            },
            '[' => {
                // println!("C: {}", code[i]);
                map.push(CMD::JmpR(0));
                i += 1;
            },
            ']' => {
                // println!("C: {}", code[i]);
                map.push(CMD::JmpL(0));
                i += 1;
            },
            _ => {
                // println!("C: {}", code[i]);
                i += 1;
            },
        }
    }
    map
}

#[derive(Debug, Clone, Copy)]
enum CMD {
    Plus(usize),
    Minus(usize),
    PtrR(usize),
    PtrL(usize),
    Push(usize),
    Pull(usize),
    JmpR(usize),
    JmpL(usize),
}

fn main() {
    // read first arg as file to string
    let data: String = read_input_file();

    const COMMANDS: [char; 8] = ['+','-','>','<','.',',','[',']'];
    let mut code_txt: Vec<char> = data.chars()
                              .collect();
    // filter code_txt
    code_txt.retain(|&c| COMMANDS.contains(&c));

    // parse into CST
    let parsed_code = parse(&mut code_txt);

    // add Jmp values to CST
    let code: Vec<CMD> = add_jmp_values(parsed_code);

    println!("CODE LEN: {:?}", code_txt.len());
    println!("CST LEN: {:?}", code.len());
    let mut ip: usize = 0;

    const MAX: usize = 2048;
    let mut mem: [u8; MAX] = [0; MAX];
    let mut mp: usize = 0;

    // let max_loop = 64000;
    // let mut loop_ct: u128 = 0;
    // instruction count
    // let mut inst_ct: u128 = 0;

    println!("--- START ---");
    let time = std::time::Instant::now();
    // let start = time.elapsed().as_nanos();
    let start = time.elapsed().as_micros();
    while ip < code.len() && mp <= MAX {
        match code[ip] {
            CMD::Plus(n) => {
                // print!("+");
                mem[mp] += n as u8;
                ip += 1;
                // inst_ct += n as u128;
            },
            CMD::Minus(n) => {
                // print!("-");
                mem[mp] -= n as u8;
                ip += 1;
                // inst_ct += n as u128;
            },
            CMD::PtrR(n) => {
                // print!(">");
                mp += n;
                ip += 1;
                // inst_ct += n as u128;
            },
            CMD::PtrL(n) => {
                // print!("<");
                mp -= n;
                ip += 1;
                // inst_ct += n as u128;
            },
            CMD::Push(n) => {
                // print!(".");
                for _ in 0..n {
                    print!("{}", char::from(mem[mp]));
                }
                ip += 1;
                // inst_ct += n as u128;
            },
            CMD::Pull(_) => {
                todo!();
                // ip += 1;
            },
            CMD::JmpR(n) => {
                // print!("[");
                // println!("![: {}[{}], IP: {}", mp, mem[mp], ip);
                if mem[mp] == 0 {
                    // println!("[: {}[{}], IP: {}", mp, mem[mp], ip);
                    // ip = brack_right(ip, &code);
                    ip = n;
                } else {
                    ip += 1;
                }
                // inst_ct += 1 as u128;
            },
            CMD::JmpL(n) => {
                // print!("]");
                // println!("!]: {}[{}], IP: {}", mp, mem[mp], ip);
                if mem[mp] != 0 {
                    // println!("]: {}[{}], IP: {}", mp, mem[mp], ip);
                    // ip = brack_left(ip, &code);
                    ip = n;
                } else {
                    ip += 1;
                }
                // inst_ct += 1 as u128;
            },
        }
        // loop_ct += 1;
        // if loop_ct == max_loop {
        //     println!("LOOP OVERFLOW: {}", max_loop);
        //     break
        // }
        // println!("IP: [{}] --- MP: [{}]", ip, mp);
        // println!("IP: [{}] --- MP: [{}] --- MEM: {:?}", ip, mp, mem);
    }
    // let end = time.elapsed().as_nanos() - start;
    let end = time.elapsed().as_micros() - start;
    println!("\n--- END ---");
    // println!("MP: [{}], IP: [{}]", mp, ip);
    println!("TIME: {}s {}ms {}μs", (end / 1000000), (end / 1000), (end % 1000 ));
    // println!("Loops: {}", loop_ct);
    // println!("Inst: {}", inst_ct);
    // println!("MEM: {:?}", mem);
}

