#!/bin/bash

# 主网部署脚本

# 确保 dfx 已安装
if ! command -v dfx &> /dev/null; then
    echo "Error: dfx is not installed. Please install it first."
    exit 1
fi

# 确认部署到主网
echo "WARNING: You are about to deploy to the IC mainnet."
echo "This will use real cycles and deploy a production version of your application."
read -p "Are you sure you want to continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Deployment cancelled."
    exit 1
fi

# 确保已登录到主网
echo "Checking dfx identity..."
dfx identity use default
dfx identity get-principal

# 创建部署目录
mkdir -p .dfx/mainnet

# 设置主网环境变量
export DFX_NETWORK=ic
export CANISTER_ID_BOLLAR_MONEY_BACKEND=$(dfx canister --network ic id bollar_money_backend 2>/dev/null || echo "")
export CANISTER_ID_BOLLAR_MONEY_FRONTEND=$(dfx canister --network ic id bollar_money_frontend 2>/dev/null || echo "")

# 检查是否已部署
if [ -z "$CANISTER_ID_BOLLAR_MONEY_BACKEND" ]; then
    echo "Canisters not yet created on mainnet. Creating new canisters..."
    
    # 创建新的 canister
    dfx canister --network ic create bollar_money_backend
    dfx canister --network ic create bollar_money_frontend
    
    # 获取 canister ID
    export CANISTER_ID_BOLLAR_MONEY_BACKEND=$(dfx canister --network ic id bollar_money_backend)
    export CANISTER_ID_BOLLAR_MONEY_FRONTEND=$(dfx canister --network ic id bollar_money_frontend)
    
    echo "Created backend canister: $CANISTER_ID_BOLLAR_MONEY_BACKEND"
    echo "Created frontend canister: $CANISTER_ID_BOLLAR_MONEY_FRONTEND"
else
    echo "Using existing canisters:"
    echo "Backend canister: $CANISTER_ID_BOLLAR_MONEY_BACKEND"
    echo "Frontend canister: $CANISTER_ID_BOLLAR_MONEY_FRONTEND"
fi

# 运行测试
echo "Running tests before deployment..."
cargo test --release

# 确认测试结果
read -p "Did all tests pass? Continue with deployment? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Deployment cancelled."
    exit 1
fi

# 构建和部署后端
echo "Building and deploying backend..."
dfx build --network ic bollar_money_backend --release
dfx canister --network ic install bollar_money_backend

# 构建和部署前端
echo "Building and deploying frontend..."
dfx build --network ic bollar_money_frontend --release
dfx canister --network ic install bollar_money_frontend

# 设置前端 canister 的访问控制
echo "Setting frontend canister access control..."
dfx canister --network ic update-settings bollar_money_frontend --controller $(dfx identity get-principal)
dfx canister --network ic update-settings bollar_money_frontend --add-controller $CANISTER_ID_BOLLAR_MONEY_BACKEND

# 设置 canister 的循环供应
echo "Setting canister cycle management..."
dfx canister --network ic update-settings bollar_money_backend --memory-allocation 4G
dfx canister --network ic update-settings bollar_money_frontend --memory-allocation 1G

# 输出访问 URL
echo "Deployment complete!"
echo "Frontend URL: https://$CANISTER_ID_BOLLAR_MONEY_FRONTEND.ic0.app/"
echo "Backend Canister ID: $CANISTER_ID_BOLLAR_MONEY_BACKEND"

# 提醒设置循环供应
echo "IMPORTANT: Remember to top up your canisters with cycles regularly."
echo "You can do this with: dfx canister --network ic deposit-cycles <amount> <canister_id>"