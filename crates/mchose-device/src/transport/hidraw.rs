//! Transporte real, sobre `/dev/hidraw`.
//!
//! Este e o unico modulo do crate com `unsafe`: os dois ioctls de feature do
//! hidraw nao tem involucro seguro na std nem em nenhuma dependencia que valha a
//! pena arrastar. O resto do crate segue sob `deny(unsafe_code)`.
#![allow(unsafe_code)]

use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

use super::Transport;

/// `_IOC(_IOC_READ|_IOC_WRITE, 'H', nr, len)`, a forma dos ioctls do hidraw.
fn ioc(nr: u32, len: usize) -> libc::c_ulong {
    const DIR_LEITURA_E_ESCRITA: u32 = 3;
    let bits =
        (DIR_LEITURA_E_ESCRITA << 30) | ((len as u32 & 0x3FFF) << 16) | ((b'H' as u32) << 8) | nr;
    libc::c_ulong::from(bits)
}

/// O descritor aberto do dispositivo.
pub(crate) struct HidrawTransport {
    fd: AsyncFd<std::fs::File>,
}

impl HidrawTransport {
    /// Abre o dispositivo ja identificado.
    ///
    /// `PermissionDenied` aqui e um estado proprio, nao "sem dongle": significa
    /// que `install/99-mchose-v9.rules` nao foi instalada, e quem mostra a
    /// mensagem precisa poder dizer *instale a regra*.
    pub(crate) fn open(caminho: &Path) -> io::Result<(Self, u64)> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let arquivo = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            // `AsyncFd` **exige** o descritor em modo nao-bloqueante: e o
            // `WouldBlock` devolvido pela leitura que diz a ele que a prontidao
            // era falsa. Sem isso, a prontidao fica em cache depois do primeiro
            // pacote e a leitura seguinte estaciona a thread do runtime.
            .custom_flags(libc::O_NONBLOCK)
            .open(caminho)?;
        let rdev = arquivo.metadata()?.rdev();
        Ok((
            Self {
                fd: AsyncFd::with_interest(arquivo, Interest::READABLE | Interest::WRITABLE)?,
            },
            rdev,
        ))
    }

    /// Chama um ioctl de feature no descritor.
    fn feature(&self, nr: u32, buf: &mut [u8]) -> io::Result<usize> {
        let req = ioc(nr, buf.len());
        // SEGURANCA: `buf` e uma fatia valida e mutavel de `buf.len()` bytes, e
        // o tamanho vai codificado no proprio numero do ioctl — o kernel nao
        // escreve alem dele. O descritor esta vivo enquanto `self` viver.
        let r = unsafe { libc::ioctl(self.fd.get_ref().as_raw_fd(), req, buf.as_mut_ptr()) };
        if r < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(r as usize)
    }
}

impl Transport for HidrawTransport {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut pronto = self.fd.readable().await?;
            match pronto.try_io(|inner| {
                use std::io::Read;
                // `&File` ja implementa Read: nao e preciso possuir o arquivo.
                let mut arquivo: &std::fs::File = inner.get_ref();
                arquivo.read(buf)
            }) {
                Ok(resultado) => return resultado,
                // Alarme falso do epoll: volta a esperar.
                Err(_seria_bloqueante) => continue,
            }
        }
    }

    async fn write(&mut self, data: &[u8]) -> io::Result<()> {
        use std::io::Write;
        let escrito = self
            .fd
            .async_io(Interest::WRITABLE, |arquivo| {
                let mut arquivo: &std::fs::File = arquivo;
                arquivo.write(data)
            })
            .await?;
        if escrito != data.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "escrita parcial no hidraw",
            ));
        }
        Ok(())
    }

    async fn get_feature(&mut self, report_id: u8, buf: &mut [u8]) -> io::Result<usize> {
        const HIDIOCGFEATURE: u32 = 0x07;
        if let Some(primeiro) = buf.first_mut() {
            *primeiro = report_id;
        }
        self.feature(HIDIOCGFEATURE, buf)
    }

    async fn set_feature(&mut self, data: &[u8]) -> io::Result<()> {
        const HIDIOCSFEATURE: u32 = 0x06;
        let mut copia = data.to_vec();
        self.feature(HIDIOCSFEATURE, &mut copia)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Caminho real, com o dongle plugado. Precisa da regra udev instalada ou
    /// de privilegio — por isso fica sob demanda.
    #[tokio::test]
    #[ignore = "precisa do dongle e de acesso ao hidraw"]
    async fn le_a_bateria_do_fone_de_verdade() {
        use mchose_protocol::battery::decode_battery;
        use mchose_protocol::request::{REPORT_LEN, battery_request};

        let achado =
            crate::discovery::find_device(Path::new("/sys/class/hidraw")).expect("dongle plugado");
        let (mut t, rdev) =
            HidrawTransport::open(&achado.path).expect("sem permissao? instale a regra udev");
        assert_eq!(
            rdev, achado.rdev,
            "o node trocou de dono entre achar e abrir"
        );
        t.write(&battery_request()).await.expect("consulta");
        let mut buf = [0u8; REPORT_LEN];
        let n = t.read(&mut buf).await.expect("resposta");
        let leitura = decode_battery(&buf[..n]).expect("pacote de bateria");
        println!("bateria: {}% {:?}", leitura.percent, leitura.state);
        assert!(leitura.percent <= 100);
    }
}
