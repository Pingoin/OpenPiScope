set dotenv-load

ssh-run cmd:
    sshpass -p$OPS_PASSWORD ssh $OPS_USER@$OPS_SERVER "{{cmd}}"

scp-upload input output:
    sshpass -p$OPS_PASSWORD scp {{input}} $OPS_USER@$OPS_SERVER:{{output}}

setup-folder:
    just ssh-run "sudo mkdir -p /opt/open-pi-scope && sudo chown -R $OPS_USER:$OPS_USER /opt/open-pi-scope"

compile:
    cross build --target aarch64-unknown-linux-gnu --release

compress: compile
    upx --best --lzma target/aarch64-unknown-linux-gnu/release/open-pi-scope

first-upload: compress
    just scp-upload "target/aarch64-unknown-linux-gnu/release/open-pi-scope" "/opt/open-pi-scope/"

upload: compress
    just ssh-run "sudo systemctl stop open-pi-scope"
    just scp-upload "target/aarch64-unknown-linux-gnu/release/open-pi-scope" "/opt/open-pi-scope/"
    just ssh-run "sudo systemctl start open-pi-scope"

install-service: first-upload
    just scp-upload "open-pi-scope.service" "/opt/open-pi-scope"
    just ssh-run "id -u openpiscope &>/dev/null || sudo useradd --system --no-create-home --shell /usr/sbin/nologin openpiscope"
    just ssh-run "sudo usermod -aG i2c openpiscope"
    just ssh-run "sudo cp /opt/open-pi-scope/open-pi-scope.service /etc/systemd/system"
    just ssh-run "sudo systemctl daemon-reload"
    just ssh-run "sudo systemctl enable open-pi-scope"
    just ssh-run "sudo systemctl start open-pi-scope"

run: upload
    just ssh-run "sudo systemctl stop open-pi-scope"
    just ssh-run "/opt/open-pi-scope/open-pi-scope"

start:
    just ssh-run "/opt/open-pi-scope/open-pi-scope"

ssh:
    just ssh-run ""

upload-config:
    just ssh-run "sudo mkdir -p /boot/open-pi-scope && sudo chown -R $OPS_USER:$OPS_USER /boot/open-pi-scope"
    just scp-upload "default-config.toml" "/boot/open-pi-scope/config.toml"