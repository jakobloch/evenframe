-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'jt5iPXhYKRbwioCukLm0vA6WFxLQgAwjzeQjrbmh1R6TvzXvxfZvgWEBmUwRythG0QjvAs2oFlTwEJ1UBwOyBwqwAqoSAfnpyDMuaLfcVIgU38uSu4LOxXyMWQzQUFPh' WITH ISSUER KEY 'jt5iPXhYKRbwioCukLm0vA6WFxLQgAwjzeQjrbmh1R6TvzXvxfZvgWEBmUwRythG0QjvAs2oFlTwEJ1UBwOyBwqwAqoSAfnpyDMuaLfcVIgU38uSu4LOxXyMWQzQUFPh' DURATION FOR TOKEN 15m, FOR SESSION 6h;

