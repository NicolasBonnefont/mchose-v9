//! Imprime a configuracao da filter-chain para um alvo dado.
//!
//! ```text
//! cargo run -p mchose-audio --example gerar -- <node.name do sink>
//! ```

fn main() {
    let alvo = std::env::args().nth(1).unwrap_or_default();
    let ganhos = [0.0f32; mchose_audio::config::BANDAS];
    match mchose_audio::config::gerar(&ganhos, &alvo) {
        Ok(texto) => print!("{texto}"),
        Err(e) => {
            eprintln!("recusado: {e:?}");
            std::process::exit(1);
        }
    }
}
