DEFINE ACCESS OVERWRITE user ON DATABASE TYPE RECORD
    SIGNUP ( CREATE user SET email = $email, password = crypto::argon2::generate($password) )
    SIGNIN ( SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password) )
    DURATION FOR TOKEN 15m, FOR SESSION 6h;
