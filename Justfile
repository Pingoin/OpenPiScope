set dotenv-load

compile:
    cross build --target aarch64-unknown-linux-gnu --release

upload: compile
    sshpass -p$OPS_PASSWORD scp target/aarch64-unknown-linux-gnu/release/open-pi-scope $OPS_USER@$OPS_SERVER:~/

run: upload
    sshpass -p$OPS_PASSWORD ssh $OPS_USER@$OPS_SERVER ./open-pi-scope