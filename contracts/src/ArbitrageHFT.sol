// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IPool} from "@aave/core-v3/contracts/interfaces/IPool.sol";
import {IPoolAddressesProvider} from "@aave/core-v3/contracts/interfaces/IPoolAddressesProvider.sol";
import {IFlashLoanSimpleReceiver} from "@aave/core-v3/contracts/flashloan/interfaces/IFlashLoanSimpleReceiver.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

// Interface standard pour le recoupement multidimensionnel sur les DEX L2
interface IUniswapV2Router {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
}

contract ArbitrageHFT is IFlashLoanSimpleReceiver {
    address public immutable owner;
    IPool public immutable aavePool;

    // Vecteurs de routage L2 (Arbitrum One)
    IUniswapV2Router public constant ROUTER_A = IUniswapV2Router(0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506); // SushiSwap V2
    IUniswapV2Router public constant ROUTER_B = IUniswapV2Router(0xc873fEcbd354f5A56E00E710B90EF4201db2448d); // Camelot

    modifier onlyOwner() {
        require(msg.sender == owner, "Rupture de securite : Acces refuse");
        _;
    }

    constructor(address _aavePool) {
        owner = msg.sender;
        aavePool = IPool(_aavePool);
    }

    // Point d'entrée asynchrone invoqué par le bot Rust (Phase 5)
    function executeArbitrage(address tokenA, address tokenB, uint256 flashAmount) external onlyOwner {
        bytes memory params = abi.encode(tokenA, tokenB);
        
        // Déclenchement du pont d'emprunt L2
        aavePool.flashLoanSimple(
            address(this),
            tokenA,
            flashAmount,
            params,
            0
        );
    }

    // Callback de validation multidimensionnelle Aave V3
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        require(msg.sender == address(aavePool), "Rupture : Protocole appelant non autorise");
        require(initiator == address(this), "Rupture : Initiateur de flux invalide");

        (address tokenA, address tokenB) = abi.decode(params, (address, address));

        // Vecteur 1 : Vente sur DEX A
        IERC20(tokenA).approve(address(ROUTER_A), amount);
        address[] memory pathA = new address[](2);
        pathA[0] = tokenA;
        pathA[1] = tokenB;

        uint[] memory amountsOutA = ROUTER_A.swapExactTokensForTokens(
            amount,
            1, // Tolerence au spread initial (absorbee par la rentabilite finale)
            pathA,
            address(this),
            block.timestamp
        );

        // Vecteur 2 : Rachat sur DEX B avec disjoncteur de rentabilité
        uint256 amountReceivedB = amountsOutA[1];
        IERC20(tokenB).approve(address(ROUTER_B), amountReceivedB);
        address[] memory pathB = new address[](2);
        pathB[0] = tokenB;
        pathB[1] = tokenA;

        // Disjoncteur L2 : Le swap doit imperativement rembourser le principal + le premium Aave
        uint256 amountToOwe = amount + premium;

        uint[] memory amountsOutB = ROUTER_B.swapExactTokensForTokens(
            amountReceivedB,
            amountToOwe, // Blocage strict du slippage destructeur
            pathB,
            address(this),
            block.timestamp
        );

        // Clôture du Flash Loan
        IERC20(asset).approve(address(aavePool), amountToOwe);

        // Extraction et transfert des benefices nets on-chain
        uint256 profit = amountsOutB[1] - amountToOwe;
        if (profit > 0) {
            IERC20(asset).transfer(owner, profit);
        }

        return true;
    }

    // Configuration d'interface requise par Aave V3 avec forçage de typage strict
    function ADDRESSES_PROVIDER() external view override returns (IPoolAddressesProvider) {
        return IPoolAddressesProvider(0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb); // Arbitrum One Provider
    }
    
    function POOL() external view override returns (IPool) {
        return aavePool;
    }

    // Module de maintenance (Sweep manuel)
    function withdrawToken(address _token) external onlyOwner {
        uint256 balance = IERC20(_token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(_token).transfer(owner, balance);
        }
    }
}
