cd ./screenshot_backend && cargo build --release && cd ..
cd ./screenshot_frontend && cargo build --release && cd ..

mkdir out
cp ./screenshot_backend/target/release/screenshot_backend ./out
cp ./screenshot_frontend/target/release/screenshot_frontend ./out
