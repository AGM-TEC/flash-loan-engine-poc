// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;
contract Trigger {
    event Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to);
    function fire() external { 
        // Simule une entrée de 1000 et une sortie de 1050 (Ratio 1.05)
        emit Swap(msg.sender, 1000, 0, 0, 1050, msg.sender); 
    }
}
