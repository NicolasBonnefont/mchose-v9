//! Descoberta do dispositivo por sysfs.
//!
//! A identificacao sai de `/sys/class/hidraw/<n>/device/uevent`, que existe para
//! qualquer dispositivo HID em qualquer barramento — USB, I2C ou `uhid`. As duas
//! alternativas foram descartadas com o hardware na mao: os atributos USB
//! (`ATTRS{idVendor}`) nao existem no nivel do hidraw e sumiriam num dispositivo
//! virtual sem pai USB, quebrando os testes; e o ioctl exigiria abrir o
//! descritor de todo hidraw da maquina — incluindo teclado, token FIDO e leitor
//! biometrico — so para perguntar de quem e.

use std::path::{Path, PathBuf};

/// O `HID_ID` do uevent tem a forma `barramento:VID:PID`, em hexadecimal.
fn parse_hid_id(valor: &str) -> Option<(u16, u16)> {
    let mut campos = valor.split(':');
    let _barramento = campos.next()?;
    let vid = u32::from_str_radix(campos.next()?.trim(), 16).ok()?;
    let pid = u32::from_str_radix(campos.next()?.trim(), 16).ok()?;
    if campos.next().is_some() {
        return None;
    }
    Some((vid as u16, pid as u16))
}

/// Le `HID_ID` e `HID_NAME` de um `uevent` ja carregado.
///
/// O nome sai como bytes: converter para `String` antes de entregar ao
/// protocolo reintroduziria o defeito do padding do buffer do ioctl.
fn identifica(uevent: &str) -> Option<(u16, u16, &[u8])> {
    let mut ids = None;
    let mut nome = None;
    for linha in uevent.lines() {
        if let Some(v) = linha.strip_prefix("HID_ID=") {
            ids = parse_hid_id(v);
        } else if let Some(v) = linha.strip_prefix("HID_NAME=") {
            nome = Some(v.as_bytes());
        }
    }
    let (vid, pid) = ids?;
    Some((vid, pid, nome?))
}

/// O dispositivo que casou.
///
/// Carrega o `rdev` porque o nome nao basta: o minor do hidraw e reciclado pelo
/// kernel, e entre identificar e abrir cabe uma reenumeracao. Comparar o `rdev`
/// depois do `open` e o que amarra os dois momentos ao mesmo dispositivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Found {
    /// O device node a abrir.
    pub(crate) path: PathBuf,
    /// `major:minor` lido do sysfs no momento da identificacao.
    pub(crate) rdev: u64,
}

/// Le `major:minor` do arquivo `dev` do sysfs.
fn parse_dev(texto: &str) -> Option<u64> {
    let (major, minor) = texto.trim().split_once(':')?;
    Some(libc::makedev(major.parse().ok()?, minor.parse().ok()?))
}

/// Procura o V9 PRO sob uma raiz de sysfs, devolvendo o device node.
///
/// `raiz` e `/sys/class/hidraw` em producao; nos testes, uma arvore falsa.
/// Nenhum descritor de dispositivo e aberto aqui: quem nao casou nunca chega a
/// ser tocado.
pub(crate) fn find_device(raiz: &Path) -> Option<Found> {
    let mut achados: Vec<(String, u64)> = Vec::new();
    for entrada in std::fs::read_dir(raiz).ok()? {
        let Ok(entrada) = entrada else { continue };
        let nome_hidraw = entrada.file_name();
        let Some(nome_hidraw) = nome_hidraw.to_str() else {
            continue;
        };
        let uevent = entrada.path().join("device").join("uevent");
        let Ok(conteudo) = std::fs::read_to_string(&uevent) else {
            continue;
        };
        let Some((vid, pid, nome)) = identifica(&conteudo) else {
            continue;
        };
        if mchose_protocol::device::is_supported(vid, pid, nome) {
            let Ok(dev) = std::fs::read_to_string(entrada.path().join("dev")) else {
                continue;
            };
            let Some(rdev) = parse_dev(&dev) else {
                continue;
            };
            achados.push((nome_hidraw.to_owned(), rdev));
        }
    }
    // Ordena para que a escolha nao dependa da ordem do diretorio, que o kernel
    // nao garante.
    achados.sort();
    let (nome, rdev) = achados.first()?;
    Some(Found {
        path: Path::new("/dev").join(nome),
        rdev: *rdev,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Monta uma arvore de sysfs falsa: um diretorio por hidraw, cada um com
    /// `device/uevent`. E a forma real — conferida em /sys/class/hidraw.
    fn sysfs(entradas: &[(&str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (i, (nome, uevent)) in entradas.iter().enumerate() {
            let base = dir.path().join(nome);
            let d = base.join("device");
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("uevent"), uevent).unwrap();
            fs::write(base.join("dev"), format!("239:{i}\n")).unwrap();
        }
        dir
    }

    const V9_PRO: &str = "DRIVER=hid-generic\n\
        HID_ID=0003:0000291D:0000385D\n\
        HID_NAME=C-Media Electronics Inc MCHOSE V9 PRO\n\
        HID_UNIQ=0123456789AB\n";

    // Mesmo VID/PID do V9 PRO: o chip C-Media reporta ids identicos.
    const S9_PRO: &str = "HID_ID=0003:0000291D:0000385D\n\
        HID_NAME=C-Media Electronics Inc MCHOSE S9 PRO\n";

    const TECLADO: &str = "HID_ID=0003:0000320F:00005000\n\
        HID_NAME=Evision RGB Keyboard\n";

    #[test]
    fn acha_o_v9_pro_entre_outros_dispositivos() {
        let dir = sysfs(&[("hidraw0", TECLADO), ("hidraw5", V9_PRO)]);
        let achado = find_device(dir.path()).expect("V9 PRO presente");
        assert_eq!(achado.path.file_name().unwrap(), "hidraw5");
    }

    #[test]
    fn ignora_irmao_de_mesmo_vid_pid() {
        let dir = sysfs(&[("hidraw3", S9_PRO)]);
        assert!(find_device(dir.path()).is_none());
    }

    #[test]
    fn uevent_malformado_ou_ausente_nao_entra_em_panico() {
        let dir = sysfs(&[
            ("hidraw0", "lixo sem campo nenhum\n"),
            ("hidraw1", "HID_ID=nao-e-hexadecimal\nHID_NAME=X\n"),
            ("hidraw2", "HID_ID=0003:0000291D\n"),
        ]);
        assert!(find_device(dir.path()).is_none());
    }

    #[test]
    fn arvore_vazia_nao_acha_nada() {
        let dir = sysfs(&[]);
        assert!(find_device(dir.path()).is_none());
    }

    /// Roda contra o sysfs real da maquina. Nao precisa de privilegio — so le
    /// texto — mas depende do dongle estar plugado, entao fica sob demanda.
    #[test]
    #[ignore = "precisa do dongle plugado"]
    fn acha_o_dongle_de_verdade() {
        let achado = find_device(std::path::Path::new("/sys/class/hidraw"));
        assert!(achado.is_some(), "dongle plugado? nada casou no sysfs real");
        println!("achado: {achado:?}");
        // Confere o rdev contra o proprio device node.
        if let Some(f) = achado {
            use std::os::unix::fs::MetadataExt;
            let meta = std::fs::metadata(&f.path).expect("device node existe");
            assert_eq!(meta.rdev(), f.rdev, "rdev do sysfs difere do device node");
        }
    }

    #[test]
    fn devolve_o_caminho_do_device_node_nao_o_do_sysfs() {
        let dir = sysfs(&[("hidraw5", V9_PRO)]);
        let achado = find_device(dir.path()).expect("presente");
        assert_eq!(achado.path, std::path::Path::new("/dev/hidraw5"));
        assert_eq!(achado.rdev, libc::makedev(239, 0));
    }
}
