//! Instala a configuracao do EQ e aplica ganhos.
//!
//! ```text
//! cargo run -p mchose-audio --example eq                  # instala com tudo em 0
//! cargo run -p mchose-audio --example eq -- 3 0 0 -2 0 0 0 0 0 4
//! ```

use mchose_audio::config::BANDAS;
use mchose_audio::{install, pipewire};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut ganhos = [0.0f32; BANDAS];
    if !args.is_empty() {
        if args.len() != BANDAS {
            eprintln!("informe {BANDAS} ganhos, ou nenhum");
            std::process::exit(2);
        }
        for (destino, texto) in ganhos.iter_mut().zip(args.iter()) {
            match texto.parse::<f32>() {
                Ok(v) => *destino = v,
                Err(_) => {
                    eprintln!("ganho invalido: {texto}");
                    std::process::exit(2);
                }
            }
        }
    }

    let Some(dump) = pipewire::dump() else {
        eprintln!("pw-dump nao respondeu — o PipeWire esta rodando?");
        std::process::exit(1);
    };
    let Some(fone) = pipewire::achar_sink_do_fone(&dump) else {
        eprintln!("sink do MCHOSE V9 PRO nao encontrado — o dongle esta plugado?");
        std::process::exit(1);
    };
    println!("fone: {} (id {})", fone.node_name, fone.id);

    let Some(dir) = install::diretorio_padrao() else {
        eprintln!("nao consegui achar o diretorio de configuracao");
        std::process::exit(1);
    };
    match install::escrever(&dir, &ganhos, &fone.node_name) {
        Ok(caminho) => println!("configuracao em {}", caminho.display()),
        Err(e) => {
            eprintln!("nao instalei: {e:?}");
            std::process::exit(1);
        }
    }

    // Instalar nao e aplicar: o no so nasce depois do restart do servico.
    match pipewire::achar_no_do_eq(&dump) {
        None => println!(
            "o sink do EQ ainda nao existe. Rode:\n  \
             systemctl --user restart filter-chain.service"
        ),
        Some(no) => {
            println!(
                "sink do EQ: id {} ({})",
                no.id,
                if no.ativo { "ativo" } else { "suspenso" }
            );
            for (i, g) in ganhos.iter().enumerate() {
                let r = pipewire::aplicar_ganho(i + 1, *g);
                println!("  banda {} -> {g:+.1} dB: {r:?}", i + 1);
            }
        }
    }
}
