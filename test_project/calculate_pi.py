#!/usr/bin/env python3
"""
Calculate PI to 100 decimal places using the Leibniz formula with series acceleration.
The formula used: pi = 12 * arctan(1/2) + 4 * arctan(1/5) + arctan(1/239)
This converges much faster than basic Leibniz series.
"""

def calculate_pi(places=100):
    """
    Calculate PI to the specified number of decimal places.
    
    Args:
        places: Number of decimal places (default 100)
    
    Returns:
        String representation of PI with specified decimal places
    """
    # Use integer arithmetic for precision
    # Scale by 10^(places + 10) for intermediate calculations
    scale = 10**(places + 10)
    
    # Calculate using Machin-like formula:
    # pi/4 = 4*arctan(1/5) - arctan(1/239)
    # arctan(x) = x - x^3/3 + x^5/5 - x^7/7 + ...
    
    def arctan_one_over(n):
        """Calculate arctan(1/n) scaled by using integer arithmetic."""
        result = 0
        term_num = scale // n
        term_denom = 1
        
        for i in range(1, scale // n + 1, 2):
            result += term_num // term_denom
            
            if i + 2 > scale // n:
                break
                
            term_num = term_num // (n * n)
            term_denom += 2
            
            result -= term_num // term_denom
            
            if i + 4 > scale // n:
                break
                
            term_num = term_num // (n * n)
            term_denom += 2
        
        return result
    
    # pi = 4 * (4*arctan(1/5) - arctan(1/239)) = 16*arctan(1/5) - 4*arctan(1/239)
    pi_scaled = 16 * arctan_one_over(5) - 4 * arctan_one_over(239)
    
    # Convert to string with appropriate decimal point
    pi_str = str(pi_scaled)
    
    # Ensure we have enough digits and add decimal point
    if len(pi_str) < places + 2:
        pi_str = pi_str.zfill(places + 2)
    
    # Insert decimal point and format
    pi_str = pi_str[0] + '.' + pi_str[1:places + 1]
    
    return pi_str


def calculate_pi_with_arctan_sum(places=100):
    """
    Alternative implementation using direct arctangent series summation.
    """
    # Scale factor for decimal precision
    scale = 10**(places + 10)
    
    def arctan(x_num, x_den, n_terms):
        """Calculate arctan(x_num/x_den) using series expansion at fixed point arithmetic."""
        result = 0
        sign = 1
        term_num = x_num * scale
        term_denom = x_den
        
        for i in range(n_terms):
            term_num = term_num // (x_den ** i * x_den)
            result += sign * term_num * scale // ((2 * i + 1) * (x_den ** (2 * i)))
            sign = -sign
        
        return result
    
    # Machin's formula: pi = 16*arctan(1/5) - 4*arctan(1/239)
    arctan_1_5 = 0
    x = 5
    for i in range(200):
        for _ in range(i):
            x = x * 5
        term = scale // x
        denom = 2 * i + 1
        if i % 2 == 0:
            arctan_1_5 += scale // denom  # simplified - not optimal but demonstrates concept
            arctan_1_5 -= scale // (denom * 5**(4*i+2))  # this is simplified
    
    return str(scale)[:10] + '.' + str(scale)[10:places+11]


def calculate_pi_simple(places=100):
    """
    Simple and reliable calculation using Machin's formula
    with careful decimal arithmetic.
    """
    from decimal import Decimal, getcontext
    
    # Set precision higher than requested for intermediate calculations
    getcontext().prec = places + 15
    
    def arctan(x, k):
        """Calculate arctan(1/x) using the first k terms of the series."""
        result = Decimal(0)
        term = Decimal(1) / Decimal(x)
        
        for n in range(1, 200, 2):
            result += term / Decimal(n)
            term = term / (Decimal(x) ** 2)
        
        return result
    
    Machin_formula = 4 * (4 * arctan(5, 200) - arctan(239, 200))
    
    return f"{Machin_formula:.{places}f}"


def main():
    """Main function to calculate and display PI."""
    decimal_places = 100
    
    print(f"Calculating PI to {decimal_places} decimal places...\n")
    
    # Using the simple method with Decimal module
    pi_value = calculate_pi_simple(decimal_places)
    
    print(f"PI = {pi_value}")
    print(f"\nNumber of decimal places: {decimal_places}")
    print(f"\nFormatted with comma separators: {pi_value}")


if __name__ == "__main__":
    main()
