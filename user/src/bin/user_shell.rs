#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

/// # shell程序
/// `user/src/bin/user_shell.rs`
//

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;
const LINE_START: &str = ">> ";

use alloc::string::String;
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{close, dup, exec, fork, open, pipe, waitpid, OpenFlags};

/// ### 命令及其参数
/// |成员变量|含义|
/// |--|--|
/// |`input`|重定向输入文件名|
/// |`output`|定向输出文件名|
/// |`args_copy`|字符串向量形式的参数向量|
/// |`args_addr`|各个参数的地址向量|
/// ```
/// pub fn new()
/// ```
#[derive(Debug)]
struct ProcessArguments {   /// 重定向输入文件名
    input: String,  /// 重定向输出文件名
    output: String, /// 字符串向量形式的参数向量
    args_copy: Vec<String>, /// 各个参数的地址向量
    args_addr: Vec<*const u8>,
}

impl ProcessArguments {
    pub fn new(command: &str) -> Self {
        let args: Vec<_> = command.split(' ').collect();    // 以空格分割命令中的参数
        let mut args_copy: Vec<String> = args   // 将参数字符数组向量转化为参数字符串向量，并在每个参数后加 `\0`
            .iter()
            .filter(|&arg| !arg.is_empty())
            .map(|&arg| {
                let mut string = String::new();
                string.push_str(arg);
                string.push('\0');
                string
            })
            .collect();

        // 处理输入重定向
        let mut input = String::new();
        if let Some((idx, _)) = args_copy
            .iter() // 在命令参数中查找 '<' 重定向标记
            .enumerate()
            .find(|(_, arg)| arg.as_str() == "<\0")
        {   // 将文件名保存到输入文件中，并从参数中删去重定向部分参数
            input = args_copy[idx + 1].clone();
            args_copy.drain(idx..=idx + 1);
        }

        // 处理输出重定向
        let mut output = String::new();
        if let Some((idx, _)) = args_copy
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == ">\0")
        {
            output = args_copy[idx + 1].clone();
            args_copy.drain(idx..=idx + 1);
        }

        let mut args_addr: Vec<*const u8> = args_copy.iter().map(|arg| arg.as_ptr()).collect();
        args_addr.push(core::ptr::null::<u8>());

        Self {
            input,
            output,
            args_copy,
            args_addr,
        }
    }
}

#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new();
    print!("{}", LINE_START);
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");   // 换行(回显)
                if !line.is_empty() {
                    let splited: Vec<_> = line.as_str().split('|').collect();   // 通过管道标识 | 分割命令及参数
                    let process_arguments_list: Vec<_> = splited       // 为每一组命令和参数对创建 命令参数实例
                        .iter()
                        .map(|&cmd| ProcessArguments::new(cmd))
                        .collect();
                    // 验证输入命令重定向的有效性，除了收尾能够I/O重定向外，中间不能I/O重定向
                    let mut valid = true;
                    for (i, process_args) in process_arguments_list.iter().enumerate() {
                        if i == 0 {
                            if !process_args.output.is_empty() {
                                valid = false;
                            }
                        } else if i == process_arguments_list.len() - 1 {
                            if !process_args.input.is_empty() {
                                valid = false;
                            }
                        } else if !process_args.output.is_empty() || !process_args.input.is_empty(){
                            valid = false;
                        }
                    }
                    if process_arguments_list.len() == 1 {
                        valid = true;
                    }

                    if !valid {
                        println!("Invalid command: Inputs/Outputs cannot be correctly binded!");
                    } else {
                        // 创建管道
                        let mut pipes_fd: Vec<[usize; 2]> = Vec::new();
                        if !process_arguments_list.is_empty() {
                            for _ in 0..process_arguments_list.len() - 1 {
                                let mut pipe_fd = [0usize; 2];
                                pipe(&mut pipe_fd);
                                pipes_fd.push(pipe_fd);
                            }
                        }
                        let mut children: Vec<_> = Vec::new();
                        for (i, process_argument) in process_arguments_list.iter().enumerate() {
                            let pid = fork();
                            if pid == 0 {   // 子进程
                                let input = &process_argument.input;
                                let output = &process_argument.output;
                                let args_copy = &process_argument.args_copy;
                                let args_addr = &process_argument.args_addr;
                                // 输入重定向
                                if !input.is_empty() {
                                    let input_fd = open(input.as_str(), OpenFlags::RDONLY);
                                    if input_fd == -1 {
                                        println!("Error when opening file {}", input);
                                        return -4;
                                    }
                                    // 关闭标准输入后立马复制一个输入文件描述符，可以做到输入重定向
                                    let input_fd = input_fd as usize;
                                    close(0);
                                    assert_eq!(dup(input_fd), 0);
                                    close(input_fd);
                                }
                                // 输出重定向
                                if !output.is_empty() {
                                    let output_fd = open(
                                        output.as_str(),
                                        OpenFlags::CREATE | OpenFlags::WRONLY,
                                    );
                                    if output_fd == -1 {
                                        println!("Error when opening file {}", output);
                                        return -4;
                                    }
                                    let output_fd = output_fd as usize;
                                    close(1);
                                    assert_eq!(dup(output_fd), 1);
                                    close(output_fd);
                                }
                                // receive input from the previous process
                                if i > 0 {
                                    close(0);
                                    let read_end = pipes_fd.get(i - 1).unwrap()[0];
                                    assert_eq!(dup(read_end), 0);
                                }
                                // send output to the next process
                                if i < process_arguments_list.len() - 1 {
                                    close(1);
                                    let write_end = pipes_fd.get(i).unwrap()[1];
                                    assert_eq!(dup(write_end), 1);
                                }
                                // close all pipe ends inherited from the parent process
                                for pipe_fd in pipes_fd.iter() {
                                    close(pipe_fd[0]);
                                    close(pipe_fd[1]);
                                }
                                // execute new application
                                if exec(args_copy[0].as_str(), args_addr.as_slice()) == -1 {
                                    println!("Error when executing!");
                                    return -4;
                                }
                                unreachable!();
                            } else {    // 父进程
                                children.push(pid);
                            }
                        }
                        for pipe_fd in pipes_fd.iter() {
                            close(pipe_fd[0]);
                            close(pipe_fd[1]);
                        }
                        let mut exit_code: i32 = 0;
                        for pid in children.into_iter() {
                            let exit_pid = waitpid(pid as usize, &mut exit_code);
                            assert_eq!(pid, exit_pid);
                            if exit_code != 0 {
                                println!("Shell: Process {} exited with code {}", pid, exit_code);
                            }
                        }
                    }
                    line.clear();
                }
                print!("{}", LINE_START);
            }
            BS | DL => {    // 用户输入退格键
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");    // 用空格覆盖原来的字符
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);    // 回显
                line.push(c as char);   // 将输入保存到line
            }
        }
    }
}
