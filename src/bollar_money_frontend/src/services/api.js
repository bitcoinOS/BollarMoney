import { Actor, HttpAgent } from '@dfinity/agent';
import { idlFactory } from '../../../declarations/bollar_money_backend/bollar_money_backend.did.js';

// 本地开发环境的 canister ID
const LOCAL_CANISTER_ID = process.env.BOLLAR_MONEY_BACKEND_CANISTER_ID;

// 生产环境的 canister ID
const PRODUCTION_CANISTER_ID = "aaaaa-aa"; // 需要替换为实际的 canister ID

// 确定当前环境
const isLocalEnv = process.env.NODE_ENV !== 'production';
const canisterId = isLocalEnv ? LOCAL_CANISTER_ID : PRODUCTION_CANISTER_ID;

// 创建 HTTP 代理
const createAgent = () => {
  const agent = new HttpAgent({
    host: isLocalEnv ? 'http://localhost:8000' : 'https://ic0.app',
  });

  // 在本地环境中获取根密钥
  if (isLocalEnv) {
    agent.fetchRootKey().catch(err => {
      console.warn('Unable to fetch root key. Check to ensure that your local replica is running');
      console.error(err);
    });
  }

  return agent;
};

// 创建 Actor
const createActor = (canisterId, options = {}) => {
  const agent = createAgent();
  return Actor.createActor(idlFactory, {
    agent,
    canisterId,
    ...options,
  });
};

// 后端 API 服务
class BollarMoneyApi {
  constructor() {
    this.actor = createActor(canisterId);
  }

  // 用户认证
  async authenticate(address, signature, message) {
    try {
      return await this.actor.authenticate(address, signature, message);
    } catch (error) {
      console.error('Authentication error:', error);
      throw error;
    }
  }

  // 获取 BTC 价格
  async getBtcPrice() {
    try {
      return await this.actor.get_btc_price();
    } catch (error) {
      console.error('Failed to get BTC price:', error);
      throw error;
    }
  }

  // 获取资金池信息
  async getPoolInfo(poolAddress) {
    try {
      return await this.actor.get_pool_info({ pool_address: poolAddress });
    } catch (error) {
      console.error('Failed to get pool info:', error);
      throw error;
    }
  }

  // 获取用户头寸列表
  async getUserPositions(user) {
    try {
      return await this.actor.get_user_positions(user);
    } catch (error) {
      console.error('Failed to get user positions:', error);
      throw error;
    }
  }

  // 获取协议指标
  async getProtocolMetrics() {
    try {
      return await this.actor.get_protocol_metrics();
    } catch (error) {
      console.error('Failed to get protocol metrics:', error);
      throw error;
    }
  }

  // 预抵押查询
  async preDeposit(poolAddress, btcAmount) {
    try {
      return await this.actor.pre_deposit(poolAddress, btcAmount);
    } catch (error) {
      console.error('Failed to pre-deposit:', error);
      throw error;
    }
  }

  // 执行抵押和铸造
  async executeDeposit(poolAddress, signedPsbt, bollarAmount) {
    try {
      return await this.actor.execute_deposit(poolAddress, signedPsbt, bollarAmount);
    } catch (error) {
      console.error('Failed to execute deposit:', error);
      throw error;
    }
  }

  // 预还款查询
  async preRepay(positionId, bollarAmount) {
    try {
      return await this.actor.pre_repay(positionId, bollarAmount);
    } catch (error) {
      console.error('Failed to pre-repay:', error);
      throw error;
    }
  }

  // 执行还款和赎回
  async executeRepay(positionId, signedPsbt) {
    try {
      return await this.actor.execute_repay(positionId, signedPsbt);
    } catch (error) {
      console.error('Failed to execute repay:', error);
      throw error;
    }
  }

  // 获取可清算头寸列表
  async getLiquidatablePositions() {
    try {
      return await this.actor.get_liquidatable_positions();
    } catch (error) {
      console.error('Failed to get liquidatable positions:', error);
      throw error;
    }
  }

  // 预清算查询
  async preLiquidate(positionId, bollarRepayAmount) {
    try {
      return await this.actor.pre_liquidate(positionId, bollarRepayAmount);
    } catch (error) {
      console.error('Failed to pre-liquidate:', error);
      throw error;
    }
  }

  // 执行清算
  async executeLiquidate(positionId, signedPsbt) {
    try {
      return await this.actor.execute_liquidate(positionId, signedPsbt);
    } catch (error) {
      console.error('Failed to execute liquidate:', error);
      throw error;
    }
  }
}

// 导出 API 实例
export const api = new BollarMoneyApi();

// 导出创建 Actor 的函数，以便在需要时创建新的 Actor
export { createActor };