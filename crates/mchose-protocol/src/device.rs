//! Identificacao do dispositivo.
//!
//! O par VID/PID **nao** identifica o produto: `291d:385d` e compartilhado por
//! S9 PRO, G9 PRO, V9 e V9 PRO, porque o chip C-Media reporta os mesmos ids
//! para produtos diferentes. Casar so por ele faz o applet falar com o fone
//! errado.

/// Vendor id do dongle 2.4GHz. Compartilhado com os irmaos.
pub const VENDOR_ID: u16 = 0x291D;
/// Product id do dongle 2.4GHz. Compartilhado com os irmaos.
pub const PRODUCT_ID: u16 = 0x385D;

/// Trecho que o nome precisa conter.
const NAME_MUST_CONTAIN: &str = "V9 PRO";
/// Trecho que desqualifica: o V9 PRO 2 usa outro caminho no firmware.
const NAME_MUST_NOT_CONTAIN: &str = "V9 PRO 2";

/// Diz se o dispositivo e o MCHOSE V9 PRO.
///
/// `product_name` vem como bytes crus porque o kernel expoe **duas formas** do
/// nome para o mesmo dispositivo: `HIDIOCGRAWNAME` devolve
/// `"C-Media Electronics Inc MCHOSE V9 PRO"` e o `ATTRS{product}` do udev
/// devolve `"MCHOSE V9 PRO"`. O card #2 enumera por ioctl e reata por udev no
/// hotplug, entao comparar por igualdade faria o applet nunca reencontrar o
/// fone depois de um replug — sem erro nenhum. Por isso o casamento e por
/// conteudo.
///
/// Bytes que nao formam UTF-8 valido nao reconhecem, e nao entram em panico:
/// o nome vem do descritor USB, ou seja, e entrada controlada pelo dispositivo.
pub fn is_supported(vid: u16, pid: u16, product_name: &[u8]) -> bool {
    if vid != VENDOR_ID || pid != PRODUCT_ID {
        return false;
    }
    let Ok(name) = str::from_utf8(product_name) else {
        return false;
    };
    // A exclusao vem ANTES da inclusao: "MCHOSE V9 PRO 2" contem "V9 PRO",
    // entao na ordem inversa a regra de exclusao seria decorativa.
    !name.contains(NAME_MUST_NOT_CONTAIN) && name.contains(NAME_MUST_CONTAIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Forma devolvida por HIDIOCGRAWNAME.
    const NOME_IOCTL: &[u8] = b"C-Media Electronics Inc MCHOSE V9 PRO";
    // Forma devolvida por ATTRS{product} do udev, no MESMO dispositivo.
    const NOME_UDEV: &[u8] = b"MCHOSE V9 PRO";

    #[test]
    fn reconhece_as_duas_formas_do_nome() {
        assert!(is_supported(VENDOR_ID, PRODUCT_ID, NOME_IOCTL));
        assert!(is_supported(VENDOR_ID, PRODUCT_ID, NOME_UDEV));
    }

    #[test]
    fn rejeita_v9_pro_2_apesar_de_conter_v9_pro() {
        assert!(!is_supported(VENDOR_ID, PRODUCT_ID, b"MCHOSE V9 PRO 2"));
        assert!(!is_supported(
            VENDOR_ID,
            PRODUCT_ID,
            b"MCHOSE V9 PRO 2 ULTRA"
        ));
    }

    #[test]
    fn rejeita_irmaos_que_dividem_o_mesmo_vid_pid() {
        assert!(!is_supported(VENDOR_ID, PRODUCT_ID, b"MCHOSE S9 PRO"));
        assert!(!is_supported(VENDOR_ID, PRODUCT_ID, b"MCHOSE G9 PRO"));
        assert!(!is_supported(VENDOR_ID, PRODUCT_ID, b"MCHOSE V9"));
    }

    #[test]
    fn rejeita_vid_pid_errado_mesmo_com_nome_certo() {
        assert!(!is_supported(0x3837, PRODUCT_ID, NOME_UDEV));
        assert!(!is_supported(VENDOR_ID, 0x6045, NOME_UDEV));
    }

    #[test]
    fn nome_vazio_ou_lixo_nao_reconhece_e_nao_entra_em_panico() {
        assert!(!is_supported(VENDOR_ID, PRODUCT_ID, b""));
        let lixo = [0xFFu8; 4096];
        assert!(!is_supported(VENDOR_ID, PRODUCT_ID, &lixo));
    }
}
