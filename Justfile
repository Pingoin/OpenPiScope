set dotenv-load

setup-folder:
    sshpass -p$OPS_PASSWORD ssh $OPS_USER@$OPS_SERVER sudo mkdir -p /opt/open-pi-scope
    sshpass -p$OPS_PASSWORD ssh $OPS_USER@$OPS_SERVER sudo chown -R $OPS_USER:$OPS_USER /opt/open-pi-scope

compile:
    cross build --target aarch64-unknown-linux-gnu --release

upload: compile
    sshpass -p$OPS_PASSWORD scp target/aarch64-unknown-linux-gnu/release/open-pi-scope $OPS_USER@$OPS_SERVER:/opt/open-pi-scope/

run: upload
    sshpass -p$OPS_PASSWORD ssh $OPS_USER@$OPS_SERVER /opt/open-pi-scope/open-pi-scope