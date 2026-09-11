//! Transporte: a fronteira entre a maquina de estados e o dispositivo.
//!
//! O trait e `pub(crate)` e nenhuma funcao publica do crate aceita `&[u8]` rumo
//! ao dispositivo. Um transporte publico de bytes crus anularia a API fechada do
//! `mchose-protocol` e voltaria a alcancar os report ids `0xED` e `0x41`, de
//! conteudo desconhecido — caminho conhecido para brickar dispositivo.
//!
//! Sao **quatro** operacoes, nao duas: o report `0xAA` e declarado Feature-only
//! no descritor (pagina `0xFF22`), e so o `0x55` tem Input/Output. Um transporte
//! de apenas ler e escrever nao alcancaria o firmware.

use std::future::Future;
use std::io;

pub(crate) trait Transport: Send {
    /// Espera o proximo report do dispositivo.
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = io::Result<usize>> + Send;

    /// Envia um output report.
    fn write(&mut self, data: &[u8]) -> impl Future<Output = io::Result<()>> + Send;

    /// Le um feature report. O byte 0 do buffer devolvido e o report id.
    fn get_feature(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + Send;

    /// Envia um feature report. O byte 0 de `data` e o report id.
    fn set_feature(&mut self, data: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
}

/// Transporte de teste. Encena o que e dificil encenar com hardware: silencio,
/// truncamento e o dongle sumindo no meio de uma leitura.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct FakeTransport {
    leituras: std::collections::VecDeque<Vec<u8>>,
    features: std::collections::HashMap<u8, Vec<u8>>,
    escrito: std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
    some: bool,
    some_no_fim: bool,
}

#[cfg(test)]
impl FakeTransport {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Enfileira um report que a proxima leitura vai devolver.
    pub(crate) fn com_leitura(mut self, dados: Vec<u8>) -> Self {
        self.leituras.push_back(dados);
        self
    }

    /// Registra a resposta de um feature report.
    pub(crate) fn com_feature(mut self, report_id: u8, dados: Vec<u8>) -> Self {
        self.features.insert(report_id, dados);
        self
    }

    /// Toda operacao passa a falhar como se o dongle tivesse sido removido.
    pub(crate) fn que_some(mut self) -> Self {
        self.some = true;
        self
    }

    /// Entrega as leituras enfileiradas e so entao some. E como uma sessao real
    /// termina: o dongle e removido depois de ter funcionado.
    pub(crate) fn que_some_depois_das_leituras(mut self) -> Self {
        self.some_no_fim = true;
        self
    }

    /// Alca para o registro de escritas, viva depois de o transporte ser
    /// consumido pela sessao.
    pub(crate) fn registro(&self) -> std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>> {
        std::sync::Arc::clone(&self.escrito)
    }

    fn registra(&self, data: &[u8]) {
        if let Ok(mut r) = self.escrito.lock() {
            r.push(data.to_vec());
        }
    }

    fn sumiu() -> io::Error {
        io::Error::new(io::ErrorKind::NotConnected, "dongle removido")
    }

    fn copia(destino: &mut [u8], origem: &[u8]) -> usize {
        let n = origem.len().min(destino.len());
        if let (Some(d), Some(o)) = (destino.get_mut(..n), origem.get(..n)) {
            d.copy_from_slice(o);
        }
        n
    }
}

#[cfg(test)]
impl Transport for FakeTransport {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.some {
            return Err(Self::sumiu());
        }
        match self.leituras.pop_front() {
            Some(dados) => Ok(Self::copia(buf, &dados)),
            None if self.some_no_fim => Err(Self::sumiu()),
            // Sem nada enfileirado o fone esta em silencio: a leitura nunca
            // resolve, e quem desiste e o prazo da maquina de estados.
            None => std::future::pending().await,
        }
    }

    async fn write(&mut self, data: &[u8]) -> io::Result<()> {
        if self.some {
            return Err(Self::sumiu());
        }
        self.registra(data);
        Ok(())
    }

    async fn get_feature(&mut self, report_id: u8, buf: &mut [u8]) -> io::Result<usize> {
        if self.some {
            return Err(Self::sumiu());
        }
        match self.features.get(&report_id) {
            Some(dados) => Ok(Self::copia(buf, dados)),
            None => Err(io::Error::new(io::ErrorKind::InvalidData, "sem feature")),
        }
    }

    async fn set_feature(&mut self, data: &[u8]) -> io::Result<()> {
        if self.some {
            return Err(Self::sumiu());
        }
        self.registra(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Capturado do hardware em 11/09/2026.
    const BATERIA: &[u8] = &[0x55, 0x65, 0x46, 0x02];
    const FIRMWARE: &[u8] = &[0xAA, 0x01, 0x00, 0x00, 0x01, 0x02];

    #[tokio::test]
    async fn fake_devolve_o_pacote_capturado() {
        let mut t = FakeTransport::new().com_leitura(BATERIA.to_vec());
        let mut buf = [0u8; 64];
        let n = t.read(&mut buf).await.expect("leitura");
        assert_eq!(&buf[..n], BATERIA);
    }

    #[tokio::test]
    async fn fake_registra_o_que_foi_escrito() {
        let mut t = FakeTransport::new();
        t.write(&[0x55, 0x65, 0x01]).await.expect("escrita");
        let reg = t.registro();
        let escrito = reg.lock().unwrap().clone();
        assert_eq!(escrito, vec![vec![0x55, 0x65, 0x01]]);
    }

    #[tokio::test]
    async fn fake_encena_silencio() {
        // Sem leitura enfileirada, a leitura nunca resolve: e o caso do fone
        // desligado ou dormindo, que a maquina de estados resolve por prazo.
        let mut t = FakeTransport::new();
        let mut buf = [0u8; 64];
        let r = tokio::time::timeout(std::time::Duration::from_millis(20), t.read(&mut buf)).await;
        assert!(r.is_err(), "silencio deveria nao resolver");
    }

    #[tokio::test]
    async fn fake_encena_truncamento() {
        let mut t = FakeTransport::new().com_leitura(vec![0x55, 0x65]);
        let mut buf = [0u8; 64];
        let n = t.read(&mut buf).await.expect("leitura curta");
        assert_eq!(n, 2);
    }

    #[tokio::test]
    async fn fake_encena_o_dongle_sumindo() {
        let mut t = FakeTransport::new().que_some();
        let mut buf = [0u8; 64];
        let e = t.read(&mut buf).await.expect_err("sumiu");
        assert_eq!(e.kind(), std::io::ErrorKind::NotConnected);
    }

    #[tokio::test]
    async fn fake_responde_o_feature_de_firmware() {
        let mut t = FakeTransport::new().com_feature(0xAA, FIRMWARE.to_vec());
        t.set_feature(&[0xAA, 0x01, 0x01]).await.expect("set");
        let mut buf = [0u8; 64];
        let n = t.get_feature(0xAA, &mut buf).await.expect("get");
        assert_eq!(&buf[..n], FIRMWARE);
    }
}
