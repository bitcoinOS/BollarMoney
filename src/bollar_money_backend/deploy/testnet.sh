#!/bin/bash

# 测试网部署脚本

# 确保 dfx 已安装
if ! command -v dfx &> /dev/null; then
    echo "Error: dfx is not installed. Please install it first."
    exit 1
fi

# 确保已登录到测试网
echo "Checking dfx identity..."
dfx identity use default
dfx identity get-principal

# 创建部署目录
mkdir -p .dfx/testnet

# 设置测试网环境变量
export DFX_NETWORK=ic
export CANISTER_ID_BOLLAR_MONEY_BACKEND=$(dfx canister --network ic id bollar_money_backend 2>/dev/null || echo "")
export CANISTER_ID_BOLLAR_MONEY_FRONTEND=$(dfx canister --network ic id bollar_money_frontend 2>/dev/null || echo "")

# 检查是否已部署
if [ -z "$CANISTER_ID_BOLLAR_MONEY_BACKEND" ]; then
    echo "Canisters not yet created on testnet. Creating new canisters..."
    
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

# 构建和部署后端
echo "Building and deploying backend..."
dfx build --network ic bollar_money_backend
dfx canister --network ic install bollar_money_backend

# 构建和部署前端
echo "Building and deploying frontend..."
dfx build --network ic bollar_money_frontend
dfx canister --network ic install bollar_money_frontend

# 设置前端 canister 的访问控制
echo "Setting frontend canister access control..."
dfx canister --network ic update-settings bollar_money_frontend --controller $(dfx identity get-principal)
dfx canister --network ic update-settings bollar_money_frontend --add-controller $CANISTER_ID_BOLLAR_MONEY_BACKEND

# 输出访问 URL
echo "Deployment complete!"
echo "Frontend URL: https://$CANISTER_ID_BOLLAR_MONEY_FRONTEND.ic0.app/"
echo "Backend Canister ID: $CANISTER_ID_BOLLAR_MONEY_BACKEND"