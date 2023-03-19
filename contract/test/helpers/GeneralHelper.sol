// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "v2-core/interfaces/IUniswapV2Pair.sol";
import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";
import "forge-std/console.sol";

library GeneralHelper {
    function getAmountOut(address inputToken, address outputToken, uint amountIn) public view returns (uint amountOut) {
        // Declare uniswapv2 types
        IUniswapV2Router02 univ2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

        (uint reserveToken0, uint reserveToken1,) = IUniswapV2Pair(getUniswapPair(inputToken, outputToken)).getReserves();

        uint reserveIn;
        uint reserveOut;

        if (inputToken < outputToken) {
            // inputToken is token0
            reserveIn = reserveToken0;
            reserveOut = reserveToken1;
        } else {
            // inputToken is token1
            reserveIn = reserveToken1;
            reserveOut = reserveToken0;
        }

        //console.log("reserveIn", reserveIn);
        //console.log("reserveOut", reserveOut);

        // Get amounts out
        amountOut = univ2Router.getAmountOut(amountIn, reserveIn, reserveOut);
    }

    function getAmountIn(address inputToken, address outputToken, uint amountOut) public view returns (uint amountIn) {
        // Declare uniswapv2 types
        IUniswapV2Router02 univ2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

        (uint reserveToken0, uint reserveToken1,) = IUniswapV2Pair(getUniswapPair(inputToken, outputToken)).getReserves();

        uint reserveIn;
        uint reserveOut;

        if (inputToken < outputToken) {
            // inputToken is token0
            reserveIn = reserveToken0;
            reserveOut = reserveToken1;
        } else {
            // inputToken is token1
            reserveIn = reserveToken1;
            reserveOut = reserveToken0;
        }

        //console.log("reserveIn", reserveIn);
        //console.log("reserveOut", reserveOut);

        // Get amounts out
        amountIn = univ2Router.getAmountIn(amountOut, reserveIn, reserveOut);
    }

    function getUniswapPair(address tokenA, address tokenB) public view returns (address pair) {
        // Declare uniswapv2 types
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);

        pair = address(IUniswapV2Pair(
            univ2Factory.getPair(address(tokenA), address(tokenB))
        ));
    }

    function getSushiSwapPair(address tokenA, address tokenB) public view returns (address pair) {
        // Declare uniswapv2 types
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac);

        pair = address(IUniswapV2Pair(
            univ2Factory.getPair(address(tokenA), address(tokenB))
        ));
    }

    // one off helper, might use later idk
    function repeatString(string calldata s, uint num) public pure returns (string memory) {
        if (num == 0) {
            return "NONE";
        }

        string memory r = s;
        for (uint i = 1; i < num; i++) {
            r = string.concat(r, s);
        }
        return r;
    }

}
