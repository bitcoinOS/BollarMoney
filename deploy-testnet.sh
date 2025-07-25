#!/bin/bash

# Bollar Money 测试网部署脚本

set -e

echo "🚀 开始部署 Bollar Money 到测试网..."

# 检查必要的工具
check_dependencies() {
    echo "📋 检查依赖..."
    
    if ! command -v dfx &> /dev/null; then
        echo "❌ dfx 未安装。请安装 DFINITY SDK。"
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        echo "❌ cargo 未安装。请安装 Rust。"
        exit 1
    fi
    
    if ! command -v npm &> /dev/null; then
        echo "❌ npm 未安装。请安装 Node.js。"
        exit 1
    fi
    
    echo "✅ 所有依赖已安装"
}

# 配置测试网环境
setup_testnet() {
    echo "🔧 配置测试网环境..."
    
    # 启动本地 IC 副本（用于测试）
    dfx start --background --clean
    
    # 或者连接到 IC 测试网
    # dfx identity use default
    # dfx ledger account-id
    
    echo "✅ 测试网环境已配置"
}

# 构建后端
build_backend() {
    echo "🔨 构建后端 canister..."
    
    # 构建 Rust canister
    cargo build --target wasm32-unknown-unknown --release --package bollar_money_backend
    
    # 生成 Candid 接口文件
    candid-extractor target/wasm32-unknown-unknown/release/bollar_money_backend.wasm > src/bollar_money_backend/bollar_money_backend.did
    
    echo "✅ 后端构建完成"
}

# 构建前端
build_frontend() {
    echo "🔨 构建前端..."
    
    cd src/bollar_money_frontend
    
    # 安装依赖
    npm install
    
    # 构建前端
    npm run build
    
    cd ../..
    
    echo "✅ 前端构建完成"
}

# 运行测试
run_tests() {
    echo "🧪 运行测试..."
    
    # 运行后端测试
    cargo test --package bollar_money_backend
    
    # 运行前端测试（如果有的话）
    # cd src/bollar_money_frontend
    # npm test
    # cd ../..
    
    echo "✅ 测试通过"
}

# 部署 canister
deploy_canisters() {
    echo "🚀 部署 canisters..."
    
    # 部署后端 canister
    dfx deploy bollar_money_backend --network local
    
    # 部署前端 canister
    dfx deploy bollar_money_frontend --network local
    
    # 获取 canister IDs
    BACKEND_CANISTER_ID=$(dfx canister id bollar_money_backend --network local)
    FRONTEND_CANISTER_ID=$(dfx canister id bollar_money_frontend --network local)
    
    echo "✅ Canisters 部署完成"
    echo "📝 后端 Canister ID: $BACKEND_CANISTER_ID"
    echo "📝 前端 Canister ID: $FRONTEND_CANISTER_ID"
}

# 初始化系统
initialize_system() {
    echo "⚙️ 初始化系统..."
    
    # 初始化 Bollar 资金池
    dfx canister call bollar_money_backend init_bollar_pool '(75, 80)' --network local
    
    # 设置初始 BTC 价格（用于测试）
    dfx canister call bollar_money_backend mock_price_update '(3000000)' --network local
    
    echo "✅ 系统初始化完成"
}

# 验证部署
verify_deployment() {
    echo "🔍 验证部署..."
    
    # 检查后端 canister 状态
    dfx canister status bollar_money_backend --network local
    
    # 检查前端 canister 状态
    dfx canister status bollar_money_frontend --network local
    
    # 测试基本功能
    echo "测试获取 BTC 价格..."
    BTC_PRICE=$(dfx canister call bollar_money_backend get_btc_price --network local)
    echo "当前 BTC 价格: $BTC_PRICE"
    
    echo "测试获取协议指标..."
    dfx canister call bollar_money_backend get_protocol_metrics --network local
    
    echo "✅ 部署验证完成"
}

# 生成部署报告
generate_report() {
    echo "📊 生成部署报告..."
    
    BACKEND_CANISTER_ID=$(dfx canister id bollar_money_backend --network local)
    FRONTEND_CANISTER_ID=$(dfx canister id bollar_money_frontend --network local)
    
    cat > deployment-report.md << EOF
# Bollar Money 测试网部署报告

## 部署信息
- 部署时间: $(date)
- 网络: 本地测试网
- 部署者: $(dfx identity whoami)

## Canister 信息
- 后端 Canister ID: $BACKEND_CANISTER_ID
- 前端 Canister ID: $FRONTEND_CANISTER_ID

## 访问链接
- 前端应用: http://localhost:8000/?canisterId=$FRONTEND_CANISTER_ID
- Candid UI: http://localhost:8000/_/candid?id=$BACKEND_CANISTER_ID

## 系统配置
- 抵押率: 75%
- 清算阈值: 80%
- 初始 BTC 价格: \$30,000

## 测试账户
请使用 Unisat 钱包连接到应用进行测试。

## 注意事项
- 这是测试网部署，仅用于开发和测试
- 不要使用真实资金进行测试
- 定期备份重要数据
EOF

    echo "✅ 部署报告已生成: deployment-report.md"
}

# 主函数
main() {
    echo "🎯 Bollar Money 测试网部署开始"
    echo "=================================="
    
    check_dependencies
    setup_testnet
    build_backend
    build_frontend
    run_tests
    deploy_canisters
    initialize_system
    verify_deployment
    generate_report
    
    echo "=================================="
    echo "🎉 Bollar Money 测试网部署完成！"
    echo ""
    echo "📱 前端应用: http://localhost:8000/?canisterId=$(dfx canister id bollar_money_frontend --network local)"
    echo "🔧 Candid UI: http://localhost:8000/_/candid?id=$(dfx canister id bollar_money_backend --network local)"
    echo ""
    echo "📋 查看完整部署报告: deployment-report.md"
}

# 错误处理
trap 'echo "❌ 部署失败！请检查错误信息。"; exit 1' ERR

# 运行主函数
main "$@"