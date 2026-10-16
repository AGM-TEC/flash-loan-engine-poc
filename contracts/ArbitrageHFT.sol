// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IPool {
    function flashLoanSimple(address receiverAddress, address asset, uint256 amount, bytes calldata params, uint16 referralCode) external;
}

interface IERC20 {
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
}

contract ArbitrageHFT {
    address public immutable owner;
    IPool public immutable aavePool;

    constructor(address _aavePool) {
        owner = msg.sender;
        aavePool = IPool(_aavePool);
    }

    // Point d'entrée déclenché par le bot Rust
    function executeArbitrage(address asset, uint256 amount, address dexTarget, bytes calldata dexData) external {
        require(msg.sender == owner, "AUTH_FAILED");
        bytes memory params = abi.encode(dexTarget, dexData);
        
        aavePool.flashLoanSimple(
            address(this),
            asset,
            amount,
            params,
            0
        );
    }

    // Fonction de rappel (Callback) imposée par Aave V3
    function executeOperation(address asset, uint256 amount, uint256 premium, address initiator, bytes calldata params) external returns (bool) {
        require(msg.sender == address(aavePool), "UNTRUSTED_CALLER");
        require(initiator == address(this), "UNTRUSTED_INITIATOR");

        (address dexTarget, bytes memory dexData) = abi.decode(params, (address, bytes));

        // Exécution de l'arbitrage sur le DEX
        (bool success, ) = dexTarget.call(dexData);
        require(success, "DEX_ROUTING_FAILED");

        // Remboursement du capital et des frais (0.05%)
        uint256 amountToRepay = amount + premium;
        IERC20(asset).approve(address(aavePool), amountToRepay);

        return true;
    }

    // Extraction des bénéfices nets post-transaction
    function extractProfit(address asset) external {
        require(msg.sender == owner, "AUTH_FAILED");
        uint256 balance = IERC20(asset).balanceOf(address(this));
        IERC20(asset).transfer(owner, balance);
    }
}
