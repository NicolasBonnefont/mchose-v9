#!/usr/bin/env python3
"""Le bateria e firmware do MCHOSE V9 PRO (291d:385d) via hidraw.
Protocolo extraido do driver oficial: report 0x55 cmd 0x65.
Uso: sudo python3 mchose_battery.py [/dev/hidrawN]"""
import fcntl, glob, os, select, struct, sys, time

VID, PID = 0x291D, 0x385D
REPORT_BATTERY, CMD_BATTERY = 0x55, 0x65
REPORT_FW = 0xAA
STATUS = {2: "descarregando", 3: "carregando", 4: "cheia", 26: "dormindo"}

def _IOC(d, t, nr, size): return (d << 30) | (size << 16) | (ord(t) << 8) | nr
HIDIOCGRAWINFO   = _IOC(2, 'H', 0x03, 8)
def HIDIOCSFEATURE(l): return _IOC(3, 'H', 0x06, l)
def HIDIOCGFEATURE(l): return _IOC(3, 'H', 0x07, l)

def find_device():
    for path in sorted(glob.glob('/dev/hidraw*')):
        try:
            fd = os.open(path, os.O_RDWR)
        except OSError:
            continue
        try:
            buf = bytearray(8)
            fcntl.ioctl(fd, HIDIOCGRAWINFO, buf, True)
            _, vid, pid = struct.unpack('<ihh', bytes(buf))
            if (vid & 0xFFFF, pid & 0xFFFF) == (VID, PID):
                return path, fd
        except OSError:
            pass
        os.close(fd)
    return None, None

def read_firmware(fd, which="dongle"):
    buf = bytearray(64)
    buf[0] = REPORT_FW
    buf[1] = 0x01
    buf[2] = 1 if which == "dongle" else 0
    try:
        fcntl.ioctl(fd, HIDIOCSFEATURE(len(buf)), buf, True)
    except OSError as e:
        return f"(envio falhou: {e})"
    time.sleep(0.3)
    out = bytearray(64)
    out[0] = REPORT_FW
    try:
        fcntl.ioctl(fd, HIDIOCGFEATURE(len(out)), out, True)
    except OSError as e:
        return f"(leitura falhou: {e})"
    raw = ' '.join(f'{b:02x}' for b in out[:8])
    if out[0] == 170 and out[1] == 1:
        return ''.join(str(out[i]) for i in range(2, 6)) + f"   [cru: {raw}]"
    return f"(prefixo inesperado) [cru: {raw}]"

def parse(buf):
    if len(buf) >= 4 and buf[0] == REPORT_BATTERY and buf[1] == CMD_BATTERY:
        return buf[2], STATUS.get(buf[3], f"desconhecido({buf[3]})")
    return None

def main():
    path = sys.argv[1] if len(sys.argv) > 1 else None
    if path:
        fd = os.open(path, os.O_RDWR)
    else:
        path, fd = find_device()
        if not fd:
            print(f"Nao achei {VID:04x}:{PID:04x} em /dev/hidraw*.")
            print("O dongle esta plugado? Rodando como root?")
            return 1
    print(f"dispositivo: {path}\n")

    print("firmware do dongle:", read_firmware(fd, "dongle"))
    print("firmware do fone:  ", read_firmware(fd, "headset"))
    print()

    req = bytearray(64)
    req[0], req[1], req[2] = REPORT_BATTERY, CMD_BATTERY, 0x01
    print(f"-> {' '.join(f'{b:02x}' for b in req[:4])} ...")
    os.write(fd, bytes(req))

    deadline = time.time() + 3
    while time.time() < deadline:
        r, _, _ = select.select([fd], [], [], deadline - time.time())
        if not r:
            break
        data = os.read(fd, 64)
        print(f"<- {' '.join(f'{b:02x}' for b in data[:8])} ...")
        got = parse(data)
        if got:
            print(f"\n>>> BATERIA: {got[0]}%   estado: {got[1]}")
            return 0
    print("\nNenhuma resposta com o prefixo esperado em 3s.")
    return 2

if __name__ == '__main__':
    sys.exit(main())
