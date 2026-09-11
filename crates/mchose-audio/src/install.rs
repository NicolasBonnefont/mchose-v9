//! Escrita do arquivo de configuração.
//!
//! `~/.config/` e diretorio de execucao — `autostart`, `systemd/user`. Por isso
//! o nome do arquivo e **constante**, nunca derivado de entrada, e a escrita
//! acontece so dentro do diretorio que o chamador informa.

use std::io;
use std::path::{Path, PathBuf};

use crate::config::{BANDAS, MARCADOR, gerar};

/// Nome do arquivo. Constante de proposito.
pub const ARQUIVO: &str = "mchose-v9-eq.conf";

/// Por que a escrita nao aconteceu.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErroInstall {
    /// A configuracao nao pode nem ser gerada.
    Config(crate::config::ErroConfig),
    /// Ja existe um arquivo com esse nome que **nao** foi escrito por nos.
    ArquivoAlheio {
        /// Qual arquivo bloqueou.
        caminho: PathBuf,
    },
    /// Falha de disco.
    Io(io::Error),
}

impl From<io::Error> for ErroInstall {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Escreve a configuração no diretório dado, de forma atômica.
///
/// Recusa-se a tocar num arquivo preexistente que nao leve o marcador do crate:
/// `effect_input.eq6` e vizinhos sao do usuario, e nunca removemos o que nao
/// escrevemos.
pub fn escrever(dir: &Path, ganhos: &[f32; BANDAS], alvo: &str) -> Result<PathBuf, ErroInstall> {
    // Gera antes de criar diretorio ou tocar em disco: entrada invalida nao
    // deixa rastro nenhum.
    let texto = gerar(ganhos, alvo).map_err(ErroInstall::Config)?;

    let destino = dir.join(ARQUIVO);
    if destino.exists() {
        let atual = std::fs::read_to_string(&destino).unwrap_or_default();
        if !atual.starts_with(MARCADOR) {
            return Err(ErroInstall::ArquivoAlheio { caminho: destino });
        }
    }

    std::fs::create_dir_all(dir)?;
    // tmp + rename: o leitor nunca ve um arquivo pela metade, e um erro no meio
    // nao deixa configuracao truncada que o PipeWire recusaria em silencio.
    // Nome imprevisivel e `create_new`: um symlink preexistente num `.tmp` de
    // nome fixo faria o `write` seguir o link e truncar o alvo — `~/.bashrc`,
    // por exemplo. O guard do marcador so protege o destino final.
    let unico = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let temporario = dir.join(format!(".{ARQUIVO}.{unico}.tmp"));
    {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporario)?;
        f.write_all(texto.as_bytes())?;
    }
    std::fs::rename(&temporario, &destino)?;
    Ok(destino)
}

/// Define os ganhos: escreve o arquivo **e então** aplica ao vivo.
///
/// É o ponto de entrada que os consumidores usam. Existe porque a invariante
/// "o arquivo é a fonte da verdade do ganho" precisa ser **estrutural**: se
/// aplicar ao vivo sem escrever, o próximo restart do serviço reverte o EQ em
/// silêncio. A ordem — arquivo primeiro — é o que garante isso mesmo quando a
/// aplicação ao vivo falha.
pub fn definir_ganhos(
    dir: &Path,
    ganhos: &[f32; BANDAS],
    alvo: &str,
) -> Result<Vec<crate::pipewire::Aplicacao>, ErroInstall> {
    escrever(dir, ganhos, alvo)?;
    let Some(no) = crate::pipewire::dump()
        .as_deref()
        .and_then(crate::pipewire::achar_no_do_eq)
    else {
        // Sem nó não há o que aplicar ao vivo: o arquivo ficou correto e o
        // serviço precisa reiniciar.
        return Ok(vec![crate::pipewire::Aplicacao::SemNo; BANDAS]);
    };
    Ok(ganhos
        .iter()
        .enumerate()
        .map(|(i, g)| crate::pipewire::aplicar_ganho(&no, i + 1, *g))
        .collect())
}

/// O diretório padrão, respeitando `XDG_CONFIG_HOME`.
pub fn diretorio_padrao() -> Option<PathBuf> {
    // `var_os` devolve `Some("")` para variavel setada e vazia, e a propria
    // especificacao XDG manda tratar isso como nao-setada. Sem exigir caminho
    // absoluto, o join viraria relativo e o arquivo nasceria no diretorio de
    // trabalho — que no script de instalacao e a arvore do repositorio.
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|h| h.join(".config"))
        })?;
    Some(base.join("pipewire").join("filter-chain.conf.d"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BANDAS;

    const ALVO: &str = "alsa_output.usb-fone-01.analog-stereo";

    fn destino() -> tempfile::TempDir {
        tempfile::tempdir().expect("tmp")
    }

    #[test]
    fn escreve_o_arquivo_com_nome_constante() {
        let d = destino();
        let ganhos = [0.0f32; BANDAS];
        let caminho = escrever(d.path(), &ganhos, ALVO).expect("deve escrever");
        assert_eq!(caminho.file_name().and_then(|n| n.to_str()), Some(ARQUIVO));
        assert!(caminho.exists());
    }

    #[test]
    fn o_arquivo_escrito_leva_o_marcador() {
        let d = destino();
        let caminho = escrever(d.path(), &[0.0; BANDAS], ALVO).expect("escreve");
        let texto = std::fs::read_to_string(&caminho).expect("le");
        assert!(texto.starts_with(crate::config::MARCADOR));
    }

    #[test]
    fn nao_sobrescreve_arquivo_que_nao_criamos() {
        let d = destino();
        let alheio = d.path().join(ARQUIVO);
        std::fs::write(&alheio, "# EQ do usuario\ncontext.modules = []\n").expect("prepara");
        let erro = escrever(d.path(), &[0.0; BANDAS], ALVO).expect_err("deve recusar");
        assert!(matches!(erro, ErroInstall::ArquivoAlheio { .. }));
        // E nao encostou no conteudo.
        let texto = std::fs::read_to_string(&alheio).expect("le");
        assert!(texto.contains("EQ do usuario"));
    }

    #[test]
    fn sobrescreve_o_proprio_arquivo() {
        let d = destino();
        escrever(d.path(), &[0.0; BANDAS], ALVO).expect("primeira");
        let mut g = [0.0f32; BANDAS];
        g[2] = 6.0;
        let caminho = escrever(d.path(), &g, ALVO).expect("segunda");
        let texto = std::fs::read_to_string(&caminho).expect("le");
        assert!(texto.contains("\"Gain\" = 6.0"));
    }

    #[test]
    fn ganho_invalido_nao_deixa_arquivo_pela_metade() {
        let d = destino();
        let mut g = [0.0f32; BANDAS];
        g[0] = 99.0;
        assert!(escrever(d.path(), &g, ALVO).is_err());
        let sobrou: Vec<_> = std::fs::read_dir(d.path())
            .expect("lista")
            .filter_map(Result::ok)
            .collect();
        assert!(sobrou.is_empty(), "nem o arquivo nem o temporario ficam");
    }

    #[test]
    fn alvo_invalido_e_recusado_antes_de_tocar_o_disco() {
        let d = destino();
        assert!(escrever(d.path(), &[0.0; BANDAS], "sink\" } plugin = \"/x.so").is_err());
        assert_eq!(std::fs::read_dir(d.path()).expect("lista").count(), 0);
    }

    #[test]
    fn atualizar_um_ganho_reescreve_so_aquela_banda() {
        let d = destino();
        let mut g = [0.0f32; BANDAS];
        g[1] = 3.0;
        escrever(d.path(), &g, ALVO).expect("primeira");
        g[1] = -3.0;
        let caminho = escrever(d.path(), &g, ALVO).expect("segunda");
        let texto = std::fs::read_to_string(&caminho).expect("le");
        assert!(texto.contains("\"Gain\" = -3.0"));
        assert!(!texto.contains("\"Gain\" = 3.0"));
    }

    #[test]
    fn diretorio_inexistente_e_criado() {
        let d = destino();
        let fundo = d.path().join("pipewire").join("filter-chain.conf.d");
        let caminho = escrever(&fundo, &[0.0; BANDAS], ALVO).expect("cria o caminho");
        assert!(caminho.exists());
    }
}
