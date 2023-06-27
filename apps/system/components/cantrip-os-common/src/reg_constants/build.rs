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

    let vc_top_hjson =
        env::var("VC_TOP_HJSON").expect("missing environment variable 'VC_TOP_HJSON'");
    println!("cargo:rerun-if-env-changed=VC_TOP_HJSON");
    build.in_file_path(vc_top_hjson).generate("vc_top.rs");

    let ml_top_hjson =
        env::var("ML_TOP_HJSON").expect("missing environment variable 'ML_TOP_HJSON'");
    println!("cargo:rerun-if-env-changed=ML_TOP_HJSON");
    build.in_file_path(ml_top_hjson).generate("ml_top.rs");
}
