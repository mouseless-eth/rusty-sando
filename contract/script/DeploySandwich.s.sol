// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import "foundry-huff/HuffDeployer.sol";
import "../src/BrainDance.sol";


contract Deployer is Script {
    
    // serachers
    function setUp() public {

    }
    function run() public{
        address sandwich = HuffDeployer.broadcast("sandwich");
        console.log(address(sandwich));
    }
}
//run bash
//forge script ./script/DeploySandwich.s.sol --rpc-url http://127.0.0.1:8545 --broadcast --private-key PRIVATE_KEY
