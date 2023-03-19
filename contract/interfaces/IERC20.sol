// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

/// @dev Interface of the ERC20 standard as defined in the EIP.
interface IERC20 {
    /// @dev Returns the amount of tokens in existence.
    function totalSupply() external view returns (uint256);

    /// @dev Returns the amount of tokens owned by `account`.
    function balanceOf(address account) external view returns (uint256);

    /// @dev Moves `amount` tokens from the caller's account to `recipient`.
    function transfer(address recipient, uint256 amount)
        external
        returns (bool);

    /// @dev Returns the remaining number of tokens that `spender` will be
    function allowance(address owner, address spender)
        external
        view
        returns (uint256);

    /// @dev Sets `amount` as the allowance of `spender` over the caller's tokens.
    function approve(address spender, uint256 amount) external returns (bool);

    /// @dev Moves `amount` tokens from `sender` to `recipient` using the
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);

    /// @dev Emitted when `value` tokens are moved from one account (`from`) to
    event Transfer(address indexed from, address indexed to, uint256 value);

    /// @dev Emitted when the allowance of a `spender` for an `owner` is set by
    event Approval(address indexed owner, address indexed spender, uint256 value);
}
