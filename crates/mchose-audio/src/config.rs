//! Geração do texto da filter-chain.
//!
//! Função pura: recebe os ganhos e o nome do sink alvo, devolve o texto. Não
//! toca em arquivo nem em PipeWire — é o pedaço testável, no mesmo padrão do
//! `mchose-protocol`.

use std::fmt::Write as _;

/// Quantas bandas o EQ tem.
pub const BANDAS: usize = 10;

/// Frequências centrais, em oitavas. Escolha nossa: a tabela de EQ do bundle do
/// fabricante e da familia `thx` e nao vale para este fone.
pub const FREQUENCIAS: [f32; BANDAS] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Largura de banda de uma oitava.
const Q: f32 = 1.41;

/// Faixa de ganho aceita, em dB.
pub const GANHO_MIN: f32 = -12.0;
/// Ver [`GANHO_MIN`].
pub const GANHO_MAX: f32 = 12.0;

/// O nome do no do sink virtual. Estavel e publico: um card de UI precisa dele
/// para ler os ganhos de volta.
pub const NODE_NAME: &str = "mchose_v9_eq";

/// Primeira linha do arquivo. E o que distingue um arquivo nosso de um do
/// usuario — nunca sobrescrevemos o que nao escrevemos.
pub const MARCADOR: &str = "# gerado por mchose-audio — nao edite a mao\n";

/// Por que a geração se recusou a produzir texto.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErroConfig {
    /// Ganho fora de [`GANHO_MIN`]`..=`[`GANHO_MAX`], ou NaN, ou infinito.
    GanhoInvalido {
        /// Qual banda.
        banda: usize,
    },
    /// O nome do sink alvo tem caractere que escaparia da sintaxe.
    AlvoInvalido,
}

/// O alvo só passa se for inteiramente `[A-Za-z0-9._:-]`.
///
/// Recusa, e **não** sanitiza: o nome vem das strings USB que o próprio
/// dispositivo declara — `manufacturer`, `product` e `serial` — e um
/// `x" } plugin = "/tmp/e.so` fecharia o bloco SPA-JSON e acrescentaria uma
/// diretiva num arquivo que o PipeWire carrega no login, fazendo-o resolver
/// `plugin` para um `.so` dentro do processo da sessão. Sanitizar deixaria a
/// dúvida de qual transformação é segura; recusar não deixa.
fn alvo_aceitavel(alvo: &str) -> bool {
    !alvo.is_empty()
        && alvo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
}

/// Monta o texto da filter-chain para os ganhos dados.
pub fn gerar(ganhos: &[f32; BANDAS], alvo: &str) -> Result<String, ErroConfig> {
    if !alvo_aceitavel(alvo) {
        return Err(ErroConfig::AlvoInvalido);
    }
    for (i, g) in ganhos.iter().enumerate() {
        if !g.is_finite() || *g < GANHO_MIN || *g > GANHO_MAX {
            return Err(ErroConfig::GanhoInvalido { banda: i });
        }
    }

    let mut nos = String::new();
    for (i, (freq, ganho)) in FREQUENCIAS.iter().zip(ganhos.iter()).enumerate() {
        let label = match i {
            0 => "bq_lowshelf",
            n if n == BANDAS - 1 => "bq_highshelf",
            _ => "bq_peaking",
        };
        let banda = i + 1;
        // `write!` em String nao falha; o `let _` evita `unwrap`.
        let _ = write!(
            nos,
            "                    {{\n\
             \x20                       type  = builtin\n\
             \x20                       name  = eq_band_{banda}\n\
             \x20                       label = {label}\n\
             \x20                       control = {{ \"Freq\" = {freq:.1} \"Q\" = {Q:.2} \"Gain\" = {ganho:.1} }}\n\
             \x20                   }}\n"
        );
    }

    let mut links = String::new();
    for i in 1..BANDAS {
        let _ = writeln!(
            links,
            "                    {{ output = \"eq_band_{i}:Out\" input = \"eq_band_{}:In\" }}",
            i + 1
        );
    }

    Ok(format!(
        "{MARCADOR}\
         # EQ de {BANDAS} bandas para o MCHOSE V9 PRO.\n\
         context.modules = [\n\
         \x20   {{ name = libpipewire-module-filter-chain\n\
         \x20       args = {{\n\
         \x20           node.description = \"MCHOSE V9 PRO EQ\"\n\
         \x20           media.name       = \"MCHOSE V9 PRO EQ\"\n\
         \x20           filter.graph = {{\n\
         \x20               nodes = [\n{nos}\
         \x20               ]\n\
         \x20               links = [\n{links}\
         \x20               ]\n\
         \x20           }}\n\
         \x20           audio.channels = 2\n\
         \x20           audio.position = [ FL FR ]\n\
         \x20           capture.props  = {{ node.name = \"{NODE_NAME}\" media.class = Audio/Sink }}\n\
         \x20           playback.props = {{ node.name = \"{NODE_NAME}_out\" node.passive = true target.object = \"{alvo}\" }}\n\
         \x20       }}\n\
         \x20   }}\n\
         ]\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALVO: &str = "alsa_output.usb-C-Media_MCHOSE_V9_PRO-01.analog-stereo";

    fn planos() -> [f32; BANDAS] {
        [0.0; BANDAS]
    }

    #[test]
    fn sao_dez_bandas_e_nove_ligacoes() {
        let texto = gerar(&planos(), ALVO).expect("entrada valida");
        assert_eq!(texto.matches("type  = builtin").count(), 10);
        assert_eq!(texto.matches("output =").count(), 9);
    }

    #[test]
    fn a_primeira_e_lowshelf_a_ultima_highshelf_e_o_meio_peaking() {
        let texto = gerar(&planos(), ALVO).expect("valida");
        assert_eq!(texto.matches("bq_lowshelf").count(), 1);
        assert_eq!(texto.matches("bq_highshelf").count(), 1);
        assert_eq!(texto.matches("bq_peaking").count(), 8);
        let low = texto.find("bq_lowshelf").expect("presente");
        let high = texto.find("bq_highshelf").expect("presente");
        assert!(low < high, "lowshelf vem antes");
    }

    #[test]
    fn as_frequencias_saem_na_ordem_declarada() {
        let texto = gerar(&planos(), ALVO).expect("valida");
        let mut pos = 0;
        for f in FREQUENCIAS {
            let agulha = format!("\"Freq\" = {f:.1}");
            let achado = texto[pos..].find(&agulha);
            assert!(achado.is_some(), "faltou {f} Hz");
            pos += achado.unwrap_or(0);
        }
    }

    #[test]
    fn o_alvo_entra_como_target_object() {
        let texto = gerar(&planos(), ALVO).expect("valida");
        assert!(
            texto.contains(&format!("target.object = \"{ALVO}\"")),
            "sem target.object o EQ processaria o sink padrao, nao o fone"
        );
    }

    #[test]
    fn o_ganho_aparece_no_control_da_banda() {
        let mut g = planos();
        g[3] = -4.5;
        let texto = gerar(&g, ALVO).expect("valida");
        assert!(texto.contains("\"Gain\" = -4.5"));
    }

    #[test]
    fn ganho_fora_da_faixa_e_recusado() {
        for ruim in [12.1_f32, -12.1, 100.0, -100.0] {
            let mut g = planos();
            g[0] = ruim;
            assert!(gerar(&g, ALVO).is_err(), "{ruim} deveria ser recusado");
        }
    }

    #[test]
    fn nan_e_infinito_sao_recusados() {
        for ruim in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut g = planos();
            g[5] = ruim;
            assert!(gerar(&g, ALVO).is_err(), "{ruim} deveria ser recusado");
        }
    }

    #[test]
    fn nome_de_sink_com_caractere_perigoso_e_recusado() {
        // O nome vem das strings USB que o proprio dispositivo declara: um
        // `x" } plugin = "/tmp/e.so` fecharia o bloco e faria o PipeWire
        // carregar um .so no processo da sessao.
        for ruim in [
            "sink\" } plugin = \"/tmp/e.so",
            "sink}",
            "sink#comentario",
            "sink\nplugin = x",
            "sink com espaco",
            "",
        ] {
            assert!(
                gerar(&planos(), ruim).is_err(),
                "{ruim:?} deveria ser recusado"
            );
        }
    }

    #[test]
    fn nome_de_sink_legitimo_e_aceito() {
        for bom in [ALVO, "effect_input.eq6", "a-b_c:d.e"] {
            assert!(gerar(&planos(), bom).is_ok(), "{bom:?} deveria passar");
        }
    }

    #[test]
    fn o_texto_leva_o_marcador_do_crate() {
        let texto = gerar(&planos(), ALVO).expect("valida");
        assert!(texto.starts_with(MARCADOR));
    }

    #[test]
    fn o_serial_do_dispositivo_nao_e_inventado_pelo_gerador() {
        // O gerador so repete o alvo que recebeu; nao busca serial em lugar
        // nenhum. Se o chamador passar um nome sem serial, nada acrescenta.
        let texto = gerar(&planos(), "sink_sem_serial").expect("valida");
        assert!(!texto.contains("0123456789AB"));
    }
}
