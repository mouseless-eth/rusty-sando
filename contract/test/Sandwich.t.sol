// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "foundry-huff/HuffDeployer.sol";
import "v3-core/interfaces/IUniswapV3Pool.sol";

import {IWETH} from "./interfaces/IWETH.sol";
import "./interfaces/IERC20.sol";
import "./helpers/GeneralHelper.sol";
import "./helpers/SandwichHelper.sol";

// Need custom interface cause USDT does not return a bool after swap
// see more here: https://github.com/d-xo/weird-erc20#missing-return-values
interface IUSDT {
    function transfer(address to, uint256 value) external;
}

contract ModSandwichV4 is Test {
    address binance8 = 0xF977814e90dA44bFA03b6295A0616a897441aceC;
    address keeperdao = 0x9a67F1940164d0318612b497E8e6038f902a00a4;

    // serachers
    address constant admin = 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf;
    address constant helper = 0x2B5AD5c4795c026514f8317c7a215E218DcCD6cF;

    IWETH weth = IWETH(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
    uint256 wethFundAmount = 1000000000 ether;
    address sandwich;

    SandwichHelper sandwichHelper;

    function setUp() public {
        sandwichHelper = new SandwichHelper();
        sandwich = HuffDeployer.deploy("sandwich");

        // fund sandwich
        weth.deposit{value: wethFundAmount}();
        weth.transfer(sandwich, wethFundAmount);

        // apparently if you want to prank addy they need to pay for gas
        // hours wasted on this :<
        payable(admin).transfer(100 ether);
        payable(helper).transfer(100 ether);
    }

    function testBreakUniswapV3Callback() public {
        vm.startPrank(address(0x69696969));

        bytes memory payload = abi.encodePacked(uint8(250)); // 0xfa = 250
        (bool s, ) = sandwich.call(payload);
        assertFalse(s, "only pools should be able to call callback");
    }

    // helper
    function _getV3PoolInfo(
        address _pool
    ) internal view returns (address token0, address token1, uint24 fee) {
        IUniswapV3Pool pool = IUniswapV3Pool(_pool);
        token0 = pool.token0();
        token1 = pool.token1();
        fee = pool.fee();
    }

    function testV3Weth1Input() public {
        address pool = 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640; // USDC - WETH
        (address token0, address token1, uint24 fee) = _getV3PoolInfo(pool);
        int256 amountIn = 1.2345678912341234 ether;

        (address inputToken, address outputToken) = (token1, token0);

        (bytes memory payload, uint256 encodedValue) = sandwichHelper
            .v3CreateSandwichPayloadWethIsInput(
                pool,
                inputToken,
                outputToken,
                fee,
                amountIn
            );

        vm.prank(admin, admin);
        (bool s, ) = address(sandwich).call{value: encodedValue}(payload);

        assertTrue(s, "calling swap failed");
    }

    function testV3Weth0Input() public {
        address pool = 0x7379e81228514a1D2a6Cf7559203998E20598346; // ETH - STETH
        (address token0, address token1, uint24 fee) = _getV3PoolInfo(pool);
        int256 amountIn = 1.2345678912341234 ether;

        (address outputToken, address inputToken) = (token1, token0);

        (bytes memory payload, uint256 encodedValue) = sandwichHelper
            .v3CreateSandwichPayloadWethIsInput(
                pool,
                inputToken,
                outputToken,
                fee,
                amountIn
            );

        vm.prank(admin, admin);
        (bool s, ) = address(sandwich).call{value: encodedValue}(payload);

        assertTrue(s, "calling swap failed");
    }

    function testV3Weth0OutputSmall() public {
        address pool = 0x7379e81228514a1D2a6Cf7559203998E20598346; // ETH - STETH
        (address token0, address token1, uint24 fee) = _getV3PoolInfo(pool);
        int256 amountIn = 1e6; // 100 usdt

        (address inputToken, address outputToken) = (token1, token0);

        // fund sandwich contract
        vm.startPrank(0xa48a523F3e0f1A9232BfE22bB6aE07Bb44bF36F1);
        IUSDT(inputToken).transfer(sandwich, uint256(amountIn));

        bytes memory payload = sandwichHelper
            .v3CreateSandwichPayloadWethIsOutput(
                pool,
                inputToken,
                outputToken,
                fee,
                amountIn
            );

        changePrank(admin);
        (bool s, ) = address(sandwich).call(payload);
        assertTrue(s, "v3 swap failed");
    }

    function testV3Weth0OutputBig() public {
        address pool = 0x64A078926AD9F9E88016c199017aea196e3899E1;
        (address token0, address token1, uint24 fee) = _getV3PoolInfo(pool);
        (address inputToken, address outputToken) = (token1, token0);

        int256 amountIn = 100000 ether; // 100000 btt

        // fund sandwich contract
        vm.startPrank(0x9277a463A508F45115FdEaf22FfeDA1B16352433);
        IUSDT(inputToken).transfer(sandwich, uint256(amountIn));

        bytes memory payload = sandwichHelper
            .v3CreateSandwichPayloadWethIsOutput(
                pool,
                inputToken,
                outputToken,
                fee,
                amountIn
            );

        changePrank(admin, admin);
        (bool s, ) = address(sandwich).call(payload);
        assertTrue(s, "calling swap failed");
    }

    function testV3Weth1OutputSmall() public {
        address pool = 0xC2e9F25Be6257c210d7Adf0D4Cd6E3E881ba25f8;
        (address token0, address token1, uint24 fee) = _getV3PoolInfo(pool);
        (address inputToken, address outputToken) = (token0, token1);
        int256 amountIn = 1e6; // 1000 dai

        // fund sandwich contract
        vm.startPrank(0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643);
        IERC20(inputToken).transfer(sandwich, uint256(amountIn));

        bytes memory payload = sandwichHelper
            .v3CreateSandwichPayloadWethIsOutput(
                pool,
                inputToken,
                outputToken,
                fee,
                amountIn
            );

        changePrank(admin, admin);
        (bool s, ) = address(sandwich).call(payload);
        assertTrue(s, "calling swap failed");
    }

    function testV3Weth1OutputBig() public {
        address pool = 0xC2e9F25Be6257c210d7Adf0D4Cd6E3E881ba25f8;
        (address token0, address token1, uint24 fee) = _getV3PoolInfo(pool);
        (address inputToken, address outputToken) = (token0, token1);
        int256 amountIn = 1e21; // 1000 dai

        // fund sandwich contract
        vm.startPrank(0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643);
        IERC20(inputToken).transfer(sandwich, uint256(amountIn));

        bytes memory payload = sandwichHelper
            .v3CreateSandwichPayloadWethIsOutput(
                pool,
                inputToken,
                outputToken,
                fee,
                amountIn
            );

        changePrank(admin, admin);
        (bool s, ) = address(sandwich).call(payload);
        assertTrue(s, "calling swap failed");
   }

    function testUnauthorized() public {
        vm.startPrank(address(0xf337babe));
        vm.deal(address(0xf337babe), 200 ether);

        string memory functionName = "recoverEth";
        bytes memory payload = abi.encodePacked(
            sandwichHelper.getJumpLabelFromSig(functionName)
        );
        (bool s, ) = sandwich.call(payload);

        assertFalse(s, "unauthorized addresses should not call recover eth");

        functionName = "recoverWeth";
        payload = abi.encodePacked(
            sandwichHelper.getJumpLabelFromSig(functionName)
        );
        (s, ) = sandwich.call(payload);

        assertFalse(
            s,
            "unauthorized addresses should not be able to call recover weth"
        );

        functionName = "seppuku";
        payload = abi.encodePacked(
            sandwichHelper.getJumpLabelFromSig(functionName)
        );
        (s, ) = sandwich.call(payload);

        assertFalse(
            s,
            "unauthorized addresses should not be able to seppuku contract"
        );
        changePrank(helper);
        (s, ) = sandwich.call(payload);
        assertTrue(s, "calling recoverEth from helper failed");
    }

    function testSepukku() public {
        vm.startPrank(helper);

        string memory functionName = "seppuku";
        bytes memory payload = abi.encodePacked(
            sandwichHelper.getJumpLabelFromSig(functionName)
        );
        (bool s, ) = sandwich.call(payload);
        assertTrue(s, "calling seppuku failed");
    }

    function testRecoverEth() public {
        vm.startPrank(helper);

        uint256 sandwichBalanceBefore = address(sandwich).balance;
        uint256 searcherBalanceBefore = address(helper).balance;

        string memory functionName = "recoverEth";
        emit log_bytes(
            abi.encodePacked(sandwichHelper.getJumpLabelFromSig(functionName))
        );
        bytes memory payload = abi.encodePacked(
            sandwichHelper.getJumpLabelFromSig(functionName)
        );
        (bool s, ) = sandwich.call(payload);
        assertTrue(s, "calling recoverEth failed");

        uint256 sandwichBalanceAfter = address(sandwich).balance;
        uint256 searcherBalanceAfter = address(helper).balance;

        // check balance change
        assertTrue(
            sandwichBalanceAfter == 0,
            "sandwich eth balance should be zero"
        );
        assertTrue(
            searcherBalanceAfter ==
                searcherBalanceBefore + sandwichBalanceBefore,
            "searcher should gain all eth from sandwich"
        );
    }

    // Test by recovering the initial funded amount
    function testRecoverWeth() public {
        vm.startPrank(helper);

        uint256 sandwichBalanceBefore = weth.balanceOf(sandwich);
        uint256 searcherBalanceBefore = weth.balanceOf(helper);

        string memory functionName = "recoverWeth";
        bytes memory payload = abi.encodePacked(
            sandwichHelper.getJumpLabelFromSig(functionName),
            sandwichBalanceBefore
        );
        (bool s, ) = sandwich.call(payload);
        assertTrue(s, "calling recoverWeth failed");

        uint256 sandwichBalanceAfter = weth.balanceOf(sandwich);
        uint256 searcherBalanceAfter = weth.balanceOf(helper);

        // check balance change
        assertTrue(
            sandwichBalanceAfter == 0,
            "sandwich weth balance should be zero"
        );
        assertTrue(
            searcherBalanceAfter ==
                searcherBalanceBefore + sandwichBalanceBefore,
            "searcher should gain all weth from sandwich"
        );
    }

    // +-------------------------------+
    // |        Generic Tests          |
    // +-------------------------------+
    // could add fuzzing / testing to test values at limits/boundary

    function testV2Weth0Input() public {
        address outputToken = 0xdAC17F958D2ee523a2206206994597C13D831ec7; // Tether
        uint256 amountIn = 1.94212341234123424 ether;

        // Pre swap checks
        uint256 wethBalanceBefore = weth.balanceOf(sandwich);
        uint256 usdtBalanceBefore = IERC20(outputToken).balanceOf(sandwich);

        uint256 actualAmountIn = (amountIn /
            sandwichHelper.wethEncodeMultiple()) *
            sandwichHelper.wethEncodeMultiple();
        uint256 amountOutFromEncoded = GeneralHelper.getAmountOut(
            address(weth),
            outputToken,
            actualAmountIn
        );
        (, , uint256 expectedAmountOut) = sandwichHelper
            .encodeNumToByteAndOffset(amountOutFromEncoded, 4, true, false);

        (bytes memory payloadV4, uint256 encodedValue) = sandwichHelper
            .v2CreateSandwichPayloadWethIsInput(outputToken, amountIn);
        vm.prank(admin);
        (bool s, ) = address(sandwich).call{value: encodedValue}(payloadV4);
        assertTrue(s);

        // Check values after swap
        uint256 wethBalanceChange = wethBalanceBefore -
            weth.balanceOf(sandwich);
        uint256 usdtBalanceChange = IERC20(outputToken).balanceOf(sandwich) -
            usdtBalanceBefore;

        assertEq(
            usdtBalanceChange,
            expectedAmountOut,
            "did not get expected usdt amount out from swap"
        );
        assertEq(
            wethBalanceChange,
            actualAmountIn,
            "unexpected amount of weth used in swap"
        );
    }

    function testV2Weth1Input() public {
        address outputToken = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48; // USDC
        uint256 amountIn = 0.942 ether;

        // Pre swap checks
        uint256 wethBalanceBefore = weth.balanceOf(sandwich);
        uint256 usdcBalanceBefore = IERC20(outputToken).balanceOf(sandwich);

        uint256 actualAmountIn = (amountIn /
            sandwichHelper.wethEncodeMultiple()) *
            sandwichHelper.wethEncodeMultiple();
        uint256 amountOutFromEncoded = GeneralHelper.getAmountOut(
            address(weth),
            outputToken,
            actualAmountIn
        );
        (, , uint256 expectedAmountOut) = sandwichHelper
            .encodeNumToByteAndOffset(amountOutFromEncoded, 4, true, false);

        (bytes memory payloadV4, uint256 encodedValue) = sandwichHelper
            .v2CreateSandwichPayloadWethIsInput(outputToken, amountIn);
        vm.prank(admin);
        (bool s, ) = address(sandwich).call{value: encodedValue}(payloadV4);
        assertTrue(s);

        // Check values after swap
        uint256 wethBalanceChange = wethBalanceBefore -
            weth.balanceOf(sandwich);
        uint256 usdcBalanceChange = IERC20(outputToken).balanceOf(sandwich) -
            usdcBalanceBefore;

        assertEq(
            usdcBalanceChange,
            expectedAmountOut,
            "did not get expected usdc amount out from swap"
        );
        assertEq(
            wethBalanceChange,
            actualAmountIn,
            "unexpected amount of weth used in swap"
        );
    }

    function testV2Weth0Output() public {
        address inputToken = 0xe53EC727dbDEB9E2d5456c3be40cFF031AB40A55; // superfarm
        uint256 amountIn = 1000000 * 10 ** 18;

        // Fund sandwich
        vm.prank(binance8);
        IUSDT(inputToken).transfer(sandwich, amountIn);

        // Pre swap checks
        uint256 wethBalanceBefore = weth.balanceOf(sandwich);
        uint256 superFarmBalanceBefore = IERC20(inputToken).balanceOf(sandwich);

        (, , uint256 actualAmountIn) = sandwichHelper.encodeNumToByteAndOffset(
            superFarmBalanceBefore,
            4,
            false,
            true
        );
        uint256 amountOutFromEncoded = GeneralHelper.getAmountOut(
            inputToken,
            address(weth),
            actualAmountIn
        );
        uint256 expectedAmountOut = (amountOutFromEncoded /
            sandwichHelper.wethEncodeMultiple()) *
            sandwichHelper.wethEncodeMultiple();

        // Perform swap
        (bytes memory payloadV4, uint256 encodedValue) = sandwichHelper
            .v2CreateSandwichPayloadWethIsOutput(inputToken, amountIn);
        emit log_bytes(payloadV4);
        vm.prank(admin);
        (bool s, ) = address(sandwich).call{value: encodedValue}(payloadV4);
        assertTrue(s, "swap failed");

        // Check values after swap
        uint256 wethBalanceChange = weth.balanceOf(sandwich) -
            wethBalanceBefore;
        uint256 superFarmBalanceChange = superFarmBalanceBefore -
            IERC20(inputToken).balanceOf(sandwich);

        assertEq(
            wethBalanceChange,
            expectedAmountOut,
            "did not get expected weth amount out from swap"
        );
        assertEq(
            superFarmBalanceChange,
            actualAmountIn,
            "unexpected amount of superFarm used in swap"
        );
    }

    function testV2Weth1Output() public {
        address inputToken = 0x6B175474E89094C44Da98b954EedeAC495271d0F; // DAI
        uint256 amountIn = 4722.366481770134 ether; // encoded as 0xFFFFFFFF0000000000

        console.log("amountIn:", amountIn);

        // Fund sandwich
        vm.prank(0x47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503);
        IERC20(inputToken).transfer(sandwich, amountIn);

        // Pre swap checks
        uint256 wethBalanceBefore = weth.balanceOf(sandwich);
        uint256 daiBalanceBefore = IERC20(inputToken).balanceOf(sandwich);

        (, , uint256 actualAmountIn) = sandwichHelper.encodeNumToByteAndOffset(
            daiBalanceBefore,
            4,
            false,
            false
        );
        uint256 amountOutFromEncoded = GeneralHelper.getAmountOut(
            inputToken,
            address(weth),
            actualAmountIn
        );
        uint256 expectedAmountOut = (amountOutFromEncoded /
            sandwichHelper.wethEncodeMultiple()) *
            sandwichHelper.wethEncodeMultiple();

        // Perform swap
        (bytes memory payload, uint256 encodedValue) = sandwichHelper
            .v2CreateSandwichPayloadWethIsOutput(inputToken, amountIn);
        emit log_bytes(payload);
        vm.prank(admin);
        (bool s, ) = address(sandwich).call{value: encodedValue}(payload);
        assertTrue(s, "swap failed");

        // Check values after swap
        uint256 wethBalanceChange = weth.balanceOf(sandwich) -
            wethBalanceBefore;
        uint256 daiBalanceChange = daiBalanceBefore -
            IERC20(inputToken).balanceOf(sandwich);

        assertEq(
            wethBalanceChange,
            expectedAmountOut,
            "did not get expected weth amount out from swap"
        );
        assertEq(
            daiBalanceChange,
            actualAmountIn,
            "unexpected amount of dai used in swap"
        );
    }
}
