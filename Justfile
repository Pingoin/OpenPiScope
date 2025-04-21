set dotenv-load

ssh-run cmd:
    sshpass -p$OPS_PASSWORD ssh $OPS_USER@$OPS_SERVER "{{cmd}}"

scp-upload input output:
    sshpass -p$OPS_PASSWORD scp {{input}} $OPS_USER@$OPS_SERVER:{{output}}

setup-folder:
    just ssh-run "sudo mkdir -p /opt/open-pi-scope && sudo chown -R $OPS_USER:$OPS_USER /opt/open-pi-scope"

compile:
    cross build --target aarch64-unknown-linux-gnu --release

upload: compile
    just scp-upload "target/aarch64-unknown-linux-gnu/release/open-pi-scope" "/opt/open-pi-scope/"

run: upload
    just ssh-run "/opt/open-pi-scope/open-pi-scope"

ssh:
    just ssh-run ""

upload-config:
    just ssh-run "sudo mkdir -p /boot/open-pi-scope && sudo chown -R $OPS_USER:$OPS_USER /boot/open-pi-scope"
    just scp-upload "default-config.toml" "/boot/open-pi-scope/config.toml"