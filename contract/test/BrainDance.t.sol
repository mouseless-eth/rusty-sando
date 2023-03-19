// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/Vm.sol";
import "forge-std/console.sol";
import { IWETH } from "../interfaces/IWETH.sol";
import "v2-core/interfaces/IUniswapV2Pair.sol";
import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";

import "../src/BrainDance.sol";

contract BrainDanceTest is Test {
    BrainDance brainDance;
    IWETH weth = IWETH(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
    IERC20 usdc = IERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
    address wethUsdcPairV2;

    IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);

    /// @notice Set up the testing suite
    function setUp() public {
        // Deposit ether into WETH Contract
        weth.deposit{value: 10e18}();

        brainDance = new BrainDance();

        wethUsdcPairV2 = _getPairV2(address(weth), address(usdc));

        // Transfer the weth to the sandwich contract
        weth.transfer(address(brainDance), 10e18);
    }

    function testSwapWethUsdcUniswapV3() public {
        address usdcWethPoolV3_05fee = 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640;
        brainDance.calculateSwapV3(2 ether, usdcWethPoolV3_05fee, address(weth), 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        uint balance = IERC20(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48).balanceOf(address(brainDance));
        console.log("balance: ", balance);
    }

    function testSwapWethUsdtUniswapV3() public {
        address wethUsdtPoolV3_30fee = 0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36;
        brainDance.calculateSwapV3(2 ether, wethUsdtPoolV3_30fee, address(weth), 0xdAC17F958D2ee523a2206206994597C13D831ec7);
        uint balance = IERC20(0xdAC17F958D2ee523a2206206994597C13D831ec7).balanceOf(address(brainDance));
        console.log("balance: ", balance);
    }

    function testSwapWethUsdcUniswapV2() public {
        (uint amountOut,) = brainDance.calculateSwapV2(2 ether, wethUsdcPairV2, address(weth), 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        console.log("amountOut:", amountOut);
    }

    function testSwapTaxedTokenUniswapV2() public {
        address wagie = 0x492baa7A6450712D4bbCCa01B87F029DEe3Ea3Ec;
        address wagieWethPair = _getPairV2(address(weth), wagie);
        (uint amountOut,) = brainDance.calculateSwapV2(2 ether, wagieWethPair, address(weth), 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        console.log("amountOut:", amountOut);
    }

    function testSwapTokenSushiSwap() public {
        (uint amountOut,) = brainDance.calculateSwapV2(2 ether, 0x397FF1542f962076d0BFE58eA045FfA2d347ACa0, address(weth), 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48);
        console.log("amountOut:", amountOut);
    }

    function testGetBrainDanceCode() public {
      bytes memory code = address(brainDance).code;
      emit log_bytes(code);
    }

    function _getPairV2(address tokenA, address tokenB) private view returns (address) {
        return univ2Factory.getPair(tokenA, tokenB);
    }
}
