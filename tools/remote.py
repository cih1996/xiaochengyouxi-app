"""Task-scoped SSH/SFTP helper. Credentials and known hosts remain in private/."""
import argparse
import json
from pathlib import Path
import paramiko

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('action', choices=['run', 'put', 'get'])
parser.add_argument('source')
parser.add_argument('target', nargs='?')
args = parser.parse_args()
access = json.loads((root / 'private/server-access.json').read_text(encoding='utf-8'))
client = paramiko.SSHClient()
known = root / 'private/known_hosts'
if known.exists():
    client.load_host_keys(str(known))
client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
client.connect(access['host'], username=access['username'], password=access['password'], timeout=20)
client.save_host_keys(str(known))
try:
    if args.action == 'run':
        command = Path(args.source).read_text(encoding='utf-8')
        _, stdout, stderr = client.exec_command(command)
        print(stdout.read().decode('utf-8', errors='replace'), end='')
        print(stderr.read().decode('utf-8', errors='replace'), end='')
        raise SystemExit(stdout.channel.recv_exit_status())
    with client.open_sftp() as sftp:
        if args.action == 'put':
            sftp.put(args.source, args.target)
            sftp.chmod(args.target, 0o600)
        else:
            sftp.get(args.source, args.target)
finally:
    client.close()
