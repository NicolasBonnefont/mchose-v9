//! Conversa com o PipeWire.
//!
//! Por processo externo com `argv` — sem `sh -c`, sem interpolacao — e o no
//! endereçado por **id numerico**, nunca por nome. Isso mantem o `unsafe` do
//! projeto confinado ao `mchose-device` e fecha o vetor de injecao de comando
//! que um nome de sink com aspas abriria.

use std::collections::BTreeMap;

use crate::config::{BANDAS, GANHO_MAX, GANHO_MIN, NODE_NAME};

/// O sink ALSA do fone.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SinkDoFone {
    /// Id do no no PipeWire.
    pub id: u32,
    /// `node.name`, que vira o `target.object` da filter-chain.
    pub node_name: String,
}

/// O no do nosso EQ.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NoDoEq {
    /// Id do no.
    pub id: u32,
    /// Ganhos atuais, por nome de controle.
    pub ganhos: BTreeMap<String, f32>,
    /// Se o no esta processando.
    ///
    /// **Informativo, nao preditivo.** Num spike com a filter-chain hospedada em
    /// instancia propria, escrever num no suspenso foi aceito e ignorado; sob o
    /// `filter-chain.service` do sistema a mesma escrita pega. Nao deduza daqui
    /// se a escrita vai valer — releia.
    pub ativo: bool,
}

/// `alsa.components` vem como `USB291d:385d`.
fn vid_pid(componentes: &str) -> Option<(u16, u16)> {
    let resto = componentes.strip_prefix("USB")?;
    let (v, p) = resto.split_once(':')?;
    Some((
        u16::from_str_radix(v.trim(), 16).ok()?,
        u16::from_str_radix(p.trim(), 16).ok()?,
    ))
}

fn nos(dump: &str) -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(dump)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
}

/// Acha o sink ALSA do fone num dump do `pw-dump`.
///
/// A identificacao e delegada ao `mchose_protocol::device::is_supported`,
/// alimentado com `alsa.components` e `alsa.card_name`. Casar so por VID/PID
/// pegaria S9 PRO, G9 PRO e V9; prefixo de nome pegaria o `V9 PRO 2`, que o
/// pack do protocolo manda excluir antes de incluir.
pub fn achar_sink_do_fone(dump: &str) -> Option<SinkDoFone> {
    for o in nos(dump) {
        // `else { continue }`, e nao `?`: o dump tem Core, Client e Device sem
        // `info.props`, e um `?` aqui abortaria a busca no primeiro deles.
        let Some(props) = o.get("info").and_then(|i| i.get("props")) else {
            continue;
        };
        // O microfone do mesmo fone tem os mesmos `alsa.*`. Sem exigir a classe,
        // o EQ sairia apontado para a entrada.
        if props.get("media.class").and_then(|v| v.as_str()) != Some("Audio/Sink") {
            continue;
        }
        let (Some(comp), Some(card), Some(nome)) = (
            props.get("alsa.components").and_then(|v| v.as_str()),
            props.get("alsa.card_name").and_then(|v| v.as_str()),
            props.get("node.name").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let Some((vid, pid)) = vid_pid(comp) else {
            continue;
        };
        if mchose_protocol::device::is_supported(vid, pid, card.as_bytes()) {
            let Some(id) = o.get("id").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            return Some(SinkDoFone {
                id: id as u32,
                node_name: nome.to_owned(),
            });
        }
    }
    None
}

/// Acha o no do nosso EQ e le os ganhos atuais.
pub fn achar_no_do_eq(dump: &str) -> Option<NoDoEq> {
    for o in nos(dump) {
        let Some(info) = o.get("info") else { continue };
        let Some(props) = info.get("props") else {
            continue;
        };
        if props.get("node.name").and_then(|v| v.as_str()) != Some(NODE_NAME) {
            continue;
        }
        // `info.state`, e nao `props["node.state"]`: o dump real nao tem esse
        // campo em props. A fixture do primeiro teste inventou o caminho, e por
        // isso `ativo` era sempre falso contra o sistema.
        let ativo = info.get("state").and_then(|v| v.as_str()) == Some("running");
        let mut ganhos = BTreeMap::new();
        if let Some(lista) = info
            .get("params")
            .and_then(|p| p.get("Props"))
            .and_then(|p| p.as_array())
        {
            for bloco in lista {
                let Some(pares) = bloco.get("params").and_then(|p| p.as_array()) else {
                    continue;
                };
                for par in pares.chunks(2) {
                    let (Some(chave), Some(valor)) = (
                        par.first().and_then(|c| c.as_str()),
                        par.get(1).and_then(serde_json::Value::as_f64),
                    ) else {
                        continue;
                    };
                    if chave.ends_with(":Gain") {
                        ganhos.insert(chave.to_owned(), valor as f32);
                    }
                }
            }
        }
        let Some(id) = o.get("id").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        return Some(NoDoEq {
            id: id as u32,
            ganhos,
            ativo,
        });
    }
    None
}

/// Monta o `argv` do `pw-cli` para escrever um ganho.
///
/// Devolve os argumentos separados de proposito: quem executa usa
/// `Command::args`, nunca uma string de shell.
///
/// `None` para banda ou ganho fora de faixa — **recusa, nao clampa**. O
/// `config::gerar` ja recusava; clampar aqui dava duas politicas para a mesma
/// entrada no mesmo crate, e fazia banda 99 virar banda 10 em silencio.
pub fn argumentos_de_escrita(no: u32, banda: usize, ganho: f32) -> Option<Vec<String>> {
    if !(1..=BANDAS).contains(&banda)
        || !ganho.is_finite()
        || !(GANHO_MIN..=GANHO_MAX).contains(&ganho)
    {
        return None;
    }
    Some(vec![
        "s".to_owned(),
        no.to_string(),
        "Props".to_owned(),
        format!("{{ params = [ \"eq_band_{banda}:Gain\" {ganho:.2} ] }}"),
    ])
}

/// Pergunta o estado ao PipeWire.
///
/// `argv` separado, sem shell. Falha de execucao vira `None` — PipeWire ausente
/// nao e erro deste crate.
pub fn dump() -> Option<String> {
    let saida = std::process::Command::new("pw-dump").output().ok()?;
    saida
        .status
        .success()
        .then(|| String::from_utf8_lossy(&saida.stdout).into_owned())
}

/// Resultado de uma tentativa de aplicar ganho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Aplicacao {
    /// Escrito e confirmado na releitura.
    Aplicado,
    /// A releitura nao confirmou o valor. Acontece quando o comando e aceito e
    /// ignorado, o que depende de como a filter-chain esta hospedada.
    NaoAplicado,
    /// O no do EQ nao existe — a configuracao foi instalada mas o servico nao
    /// reiniciou, ou o texto foi recusado em silencio.
    SemNo,
    /// Banda ou ganho fora de faixa. Nada foi escrito.
    Recusado,
}

/// Aplica um ganho e **confere relendo**.
///
/// A releitura vem depois da escrita, nunca antes: entre checar e escrever o no
/// pode mudar de estado. O resultado sai do valor relido, nunca deduzido do
/// estado — ver [`NoDoEq::ativo`].
pub fn aplicar_ganho(no: &NoDoEq, banda: usize, ganho: f32) -> Aplicacao {
    let Some(args) = argumentos_de_escrita(no.id, banda, ganho) else {
        return Aplicacao::Recusado;
    };
    if std::process::Command::new("pw-cli")
        .args(args)
        .output()
        .is_err()
    {
        return Aplicacao::NaoAplicado;
    }
    let chave = format!("eq_band_{banda}:Gain");
    let depois = dump().as_deref().and_then(achar_no_do_eq);
    match depois.and_then(|n| n.ganhos.get(&chave).copied()) {
        Some(v) if (v - ganho).abs() < 0.05 => Aplicacao::Aplicado,
        Some(_) => Aplicacao::NaoAplicado,
        None => Aplicacao::SemNo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorte do formato real do `pw-dump`, com os campos que importam.
    const DUMP: &str = r#"[
      { "id": 40, "type": "PipeWire:Interface:Node",
        "info": { "props": {
          "node.name": "alsa_output.usb-C-Media_Electronics_Inc_MCHOSE_V9_PRO_0123456789AB-01.analog-stereo",
          "alsa.components": "USB291d:385d",
          "alsa.card_name": "MCHOSE V9 PRO", "media.class": "Audio/Sink" } } },
      { "id": 41, "type": "PipeWire:Interface:Node",
        "info": { "props": {
          "node.name": "alsa_output.usb-outro-01.analog-stereo",
          "alsa.components": "USB291d:385d",
          "alsa.card_name": "MCHOSE V9 PRO 2", "media.class": "Audio/Sink" } } },
      { "id": 42, "type": "PipeWire:Interface:Node",
        "info": { "props": { "node.name": "mchose_v9_eq" }, "state": "running",
          "params": { "Props": [
            { "volume": 1.0 },
            { "params": [ "eq_band_1:Gain", 0.0, "eq_band_5:Gain", -7.5 ] } ] } } }
    ]"#;

    /// O dump real comeca com Core, Client e Device — objetos sem `info.props`.
    /// Um `?` dentro do laco abortaria a busca no primeiro deles.
    #[test]
    fn objetos_sem_props_nao_abortam_a_busca() {
        let com_lixo = DUMP.replacen(
            "[",
            "[ { \"id\": 0, \"type\": \"PipeWire:Interface:Core\" },\n              { \"id\": 61, \"type\": \"PipeWire:Interface:Device\", \"info\": { \"props\":              { \"alsa.components\": \"USB291d:385d\", \"alsa.card_name\": \"MCHOSE V9 PRO\" } } },",
            1,
        );
        assert_eq!(achar_sink_do_fone(&com_lixo).map(|s| s.id), Some(40));
    }

    /// O microfone do mesmo fone tem `alsa.components` e `alsa.card_name`
    /// identicos: sem exigir a classe, o EQ sairia apontado para a entrada.
    #[test]
    fn o_microfone_do_mesmo_fone_nao_casa() {
        let so_entrada = DUMP.replace(
            "\"media.class\": \"Audio/Sink\"",
            "\"media.class\": \"Audio/Source\"",
        );
        assert!(achar_sink_do_fone(&so_entrada).is_none());
    }

    #[test]
    fn acha_o_sink_do_fone_pelo_is_supported() {
        let achado = achar_sink_do_fone(DUMP).expect("o V9 PRO esta no dump");
        assert_eq!(achado.id, 40);
        assert!(achado.node_name.contains("MCHOSE_V9_PRO"));
    }

    #[test]
    fn o_irmao_v9_pro_2_nao_casa() {
        // Mesmo VID/PID; quem separa e o nome, e a exclusao vem antes da
        // inclusao — regra do pack do protocolo, nao reimplementada aqui.
        let so_o_irmao = DUMP.replace("\"id\": 40", "\"id\": 400").replacen(
            "MCHOSE V9 PRO\"",
            "MCHOSE V9 PRO 2\"",
            1,
        );
        assert!(achar_sink_do_fone(&so_o_irmao).is_none());
    }

    #[test]
    fn dump_sem_o_fone_nao_acha_nada() {
        assert!(achar_sink_do_fone("[]").is_none());
        assert!(achar_sink_do_fone("nao e json").is_none());
    }

    #[test]
    fn acha_o_no_do_eq_e_le_os_ganhos() {
        let eq = achar_no_do_eq(DUMP).expect("o eq esta no dump");
        assert_eq!(eq.id, 42);
        assert_eq!(eq.ganhos.get("eq_band_5:Gain"), Some(&-7.5));
        assert_eq!(eq.ganhos.get("eq_band_1:Gain"), Some(&0.0));
    }

    #[test]
    fn o_estado_do_no_e_lido() {
        let eq = achar_no_do_eq(DUMP).expect("presente");
        assert!(eq.ativo, "running conta como ativo");
    }

    #[test]
    fn no_suspenso_nao_conta_como_ativo() {
        let suspenso = DUMP.replace("\"state\": \"running\"", "\"state\": \"suspended\"");
        let eq = achar_no_do_eq(&suspenso).expect("presente");
        assert!(!eq.ativo);
    }

    #[test]
    fn o_comando_de_escrita_nao_passa_por_shell() {
        let args = argumentos_de_escrita(42, 5, -7.5).expect("entrada valida");
        // argv separado: nada de `sh -c`, nada de interpolacao. O no vai por id
        // numerico, entao nome de sink com aspas nao alcanca lugar nenhum.
        assert_eq!(args[0], "s");
        assert_eq!(args[1], "42");
        assert_eq!(args[2], "Props");
        assert!(args[3].contains("eq_band_5:Gain"));
        assert!(args[3].contains("-7.5"));
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn banda_ou_ganho_fora_de_faixa_nao_gera_comando() {
        assert!(argumentos_de_escrita(1, 0, 0.0).is_none(), "banda 0");
        assert!(argumentos_de_escrita(1, 99, 0.0).is_none(), "banda 99");
        assert!(argumentos_de_escrita(1, 5, 20.0).is_none(), "ganho alto");
        assert!(argumentos_de_escrita(1, 5, f32::NAN).is_none(), "NaN");
        assert!(argumentos_de_escrita(1, 5, 6.0).is_some(), "caso valido");
    }
}
