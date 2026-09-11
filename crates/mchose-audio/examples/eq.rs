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
    // O node.name carrega o serial do dispositivo, e esta saida e a que o
    // usuario cola numa issue do repositorio publico.
    println!("fone encontrado (id {})", fone.id);

    let Some(dir) = install::diretorio_padrao() else {
        eprintln!("nao consegui achar o diretorio de configuracao");
        std::process::exit(1);
    };
    match install::definir_ganhos(&dir, &ganhos, &fone.node_name) {
        Err(e) => {
            eprintln!("nao instalei: {e:?}");
            std::process::exit(1);
        }
        Ok(resultados) => {
            println!("configuracao em {}", dir.join(install::ARQUIVO).display());
            if resultados.iter().all(|r| *r == pipewire::Aplicacao::SemNo) {
                println!(
                    "o sink do EQ ainda nao existe. Rode:
  \
                     systemctl --user restart filter-chain.service"
                );
            } else {
                for (i, r) in resultados.iter().enumerate() {
                    let g = ganhos.get(i).copied().unwrap_or_default();
                    println!("  banda {} -> {g:+.1} dB: {r:?}", i + 1);
                }
            }
        }
    }
}
