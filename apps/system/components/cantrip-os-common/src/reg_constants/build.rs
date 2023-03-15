use std::env;

fn main() {
    let mut build = regtool::Build::new();

    let timer_hjson = env::var("TIMER_HJSON").expect("missing environment variable 'TIMER_HJSON'");
    println!("cargo:rerun-if-env-changed=TIMER_HJSON");
    build.in_file_path(timer_hjson).generate("timer.rs");

    let uart_hjson = env::var("UART_HJSON").expect("missing environment variable 'UART_HJSON'");
    println!("cargo:rerun-if-env-changed=UART_HJSON");
    build.in_file_path(uart_hjson).generate("uart.rs");

    let mbox_hjson = env::var("MBOX_HJSON").expect("missing environment variable 'MBOX_HJSON'");
    println!("cargo:rerun-if-env-changed=MBOX_HJSON");
    build.in_file_path(mbox_hjson).generate("mailbox.rs");
}
